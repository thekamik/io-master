//! **The only way to an AI service** (user, 2026-10-29): every question of
//! the program goes through these three steps, and the answer is read only
//! after it is written into the journal of the AI, so that no caller can
//! skip the journal.
//!
//! ```text
//! prepare(db, Question)  ──► Prepared   the integration, its key, the body; the size checked
//! send(&Prepared, pieces) ──► Sent       the request; the pieces of the answer into the channel
//! record(db, Prepared, Sent) ──► Answer  the entry of the journal written; the answer given out
//! ```
//!
//! The steps are apart because the database is not used while the answer
//! is awaited: the caller holds the lock of the session (and so the
//! database) only for [`prepare`] and [`record`], so that the other
//! commands of the window are not held up by a long answer.

use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::mpsc;
use zeroize::Zeroizing;

use crate::ai::client::{self, Received, Request};
use crate::ai::conversation::{Turn, turns};
use crate::ai::error::AiError;
use crate::ai::{anthropic, openai};
use crate::db::ai::api_key::read_api_key;
use crate::db::ai::integration::read_integration;
use crate::db::ai::journal::create_journal_entry;
use crate::db::database::Database;
use crate::model::ai::chat::{Message, Speaker};
use crate::model::ai::integration::{Integration, Protocol};
use crate::model::ai::journal::{Ending, JournalEntry};

/// The longest answer asked for, in tokens: a fourth of the context of the
/// model, at most this.
const MAX_ANSWER_TOKENS: u32 = 16_000;

/// The characters of a token, as the size of a question is estimated
/// before it is sent: three is fewer than most models use (about four for
/// English), so the estimate errs on the side of a question that does not
/// fit, which is refused rather than shortened by the service in silence.
const CHARACTERS_PER_TOKEN: u64 = 3;

/// The most messages of one question.
pub const MAX_MESSAGES: usize = 1000;

/// A question to ask.
pub struct Question {
    /// The integration asked.
    pub integration: i64,
    /// The agent that asks; `None` without one.
    pub agent: Option<i64>,
    /// The work unit the question is for; `None` outside an evaluation.
    pub work_unit: Option<i64>,
    /// The instructions: who the model is and how it answers.
    pub system: String,
    /// The conversation; its last message is the user's.
    pub messages: Vec<Message>,
}

/// A question ready to be sent, with what its entry of the journal needs.
pub struct Prepared {
    request: Request,
    integration: Integration,
    agent: Option<i64>,
    work_unit: Option<i64>,
    time: i64,
}

/// The answer as it came, not written into the journal yet: only
/// [`record`] gives it out.
#[must_use = "an answer is read only after record() writes it into the journal"]
pub struct Sent(Received);

/// The answer, written into the journal.
#[derive(Debug)]
pub struct Answer {
    /// The id of its entry in the journal.
    pub journal: i64,
    /// The text written; may be partial (see `ending`).
    pub text: String,
    /// The model that answered.
    pub model: String,
    /// How it ended.
    pub ending: Ending,
    /// Why it failed, when `ending` is `Failed`.
    pub error: Option<AiError>,
    /// The tokens of the question and of the answer, as the service counted
    /// them.
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// The time now, in seconds since 1970 (UTC).
///
/// Called by: [`prepare`].
fn now() -> Result<i64, AiError> {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| AiError::UnexpectedAnswer(format!("the clock: {error}")))?
        .as_secs();

    i64::try_from(seconds).map_err(|error| AiError::UnexpectedAnswer(format!("the clock: {error}")))
}

/// The longest answer asked of the integration: a fourth of its context,
/// at most [`MAX_ANSWER_TOKENS`], at least one token.
///
/// Called by: [`prepare`].
fn answer_tokens(integration: &Integration) -> u32 {
    (integration.context_size / 4).clamp(1, MAX_ANSWER_TOKENS)
}

/// Refuses a question that does not fit into the context of the model
/// beside its answer (the estimate of [`CHARACTERS_PER_TOKEN`]).
///
/// Called by: [`prepare`].
fn check_size(integration: &Integration, system: &str, turns: &[Turn]) -> Result<(), AiError> {
    let mut characters = system.chars().count();

    for turn in turns {
        characters += turn.text.chars().count();
    }

    let estimated = u64::try_from(characters).unwrap_or(u64::MAX).div_ceil(CHARACTERS_PER_TOKEN);
    let room = u64::from(integration.context_size.saturating_sub(answer_tokens(integration)));

    ensure!(estimated <= room, AiError::TooLong { estimated, room });

    Ok(())
}

/// Prepares the question `question`: reads its integration and its key,
/// makes the body in the protocol of the integration and checks that it
/// fits. Refused when there is nothing to ask, the address is not one the
/// program sends to, or the protocol needs a key the integration has not.
///
/// Called by: `panels/logic/` (every question to the AI), under the lock of
/// the session.
/// Calls: `read_integration`, `read_api_key`, `client::check_address`,
/// `conversation::turns`, [`check_size`], `openai::body`,
/// `anthropic::body`.
pub fn prepare(db: &Database, question: &Question) -> Result<Prepared, AiError> {
    ensure!(question.messages.len() <= MAX_MESSAGES, AiError::TooLong { estimated: u64::MAX, room: 0 });

    let turns = turns(&question.messages);

    ensure!(matches!(turns.last(), Some(last) if last.speaker == Speaker::User), AiError::NoQuestion);

    let integration = read_integration(db, question.integration)?;

    client::check_address(&integration.url)?;

    let key = read_api_key(db, integration.id)?;

    ensure!(key.is_some() || integration.protocol == Protocol::OpenAiChat, AiError::NoKey);
    check_size(&integration, &question.system, &turns)?;

    let body = match integration.protocol {
        Protocol::OpenAiChat => openai::body(&integration.model, &question.system, &turns),
        Protocol::AnthropicMessages => {
            anthropic::body(&integration.model, &question.system, &turns, answer_tokens(&integration))
        }
    };
    let request = Request {
        url: integration.url.clone(),
        ca_certificate: integration.ca_certificate.clone(),
        protocol: integration.protocol,
        key: key.map(|key| Zeroizing::new(key.key.as_str().to_owned())),
        body: body.to_string(),
    };

    Ok(Prepared { request, integration, agent: question.agent, work_unit: question.work_unit, time: now()? })
}

/// Sends the question and passes the pieces of the answer into `pieces`
/// while it is written; stops when the receiver of `pieces` is dropped.
/// Uses no database.
///
/// Called by: `panels/views/` (a question of a panel), the lock of the
/// session released.
/// Calls: `client::send`.
pub async fn send(prepared: &Prepared, pieces: &mpsc::Sender<String>) -> Sent {
    Sent(client::send(&prepared.request, pieces).await)
}

/// Writes the question and its answer into the journal of the AI, and
/// gives the answer out. On a failure the explanation is written after the
/// text of the answer, so that the journal tells why it failed.
///
/// Called by: `panels/logic/` (every question to the AI), under the lock of
/// the session.
/// Calls: `create_journal_entry`.
pub fn record(db: &Database, prepared: Prepared, sent: Sent) -> Result<Answer, AiError> {
    let Sent(received) = sent;
    let reading = received.reading;
    let ending = match (&received.error, received.stopped, reading.ending) {
        (Some(_), _, _) => Ending::Failed,
        (None, true, _) => Ending::Stopped,
        (None, false, ending) => ending.unwrap_or(Ending::Complete),
    };
    let model = reading.model.unwrap_or_else(|| prepared.integration.model.clone());
    let mut written = reading.text.clone();

    if let Some(error) = &received.error {
        written.push_str("\n\n[the question failed: ");
        written.push_str(&error.to_string());
        written.push(']');
    }

    let entry = JournalEntry {
        id: 0,
        time: prepared.time,
        integration: prepared.integration.name,
        model: model.clone(),
        agent: prepared.agent,
        work_unit: prepared.work_unit,
        request: prepared.request.body,
        answer: written,
        ending,
        input_tokens: reading.input_tokens,
        output_tokens: reading.output_tokens,
    };
    let journal = create_journal_entry(db, &entry)?;

    Ok(Answer {
        journal,
        text: reading.text,
        model,
        ending,
        error: received.error,
        input_tokens: reading.input_tokens,
        output_tokens: reading.output_tokens,
    })
}

#[cfg(test)]
mod tests {
    //! Tests of the way to the AI: a question prepared, sent to the fake
    //! service and written into the journal.

    use super::*;

    use crate::db::ai::api_key::{create_api_key, update_api_key};
    use crate::db::ai::integration::{create_integration, update_integration};
    use crate::db::ai::journal::read_journal_entry;
    use crate::model::ai::api_key::ApiKey;
    use crate::test_support::ai_service::{FakeAnswer, FakeService, anthropic_events, openai_events};
    use crate::test_support::database::{database, integration};

    fn question(integration: i64, text: &str) -> Question {
        Question {
            integration,
            agent: None,
            work_unit: Some(7),
            system: "Be brief.".to_owned(),
            messages: vec![Message { speaker: Speaker::User, text: text.to_owned(), time: 0 }],
        }
    }

    /// Asks through the three steps, the pieces drained.
    async fn ask(db: &Database, question: &Question) -> Result<Answer, AiError> {
        let prepared = prepare(db, question)?;
        let (sender, mut receiver) = mpsc::channel(16);
        let draining = tokio::spawn(async move { while receiver.recv().await.is_some() {} });
        let sent = send(&prepared, &sender).await;

        drop(sender);
        draining.await.unwrap();
        record(db, prepared, sent)
    }

    #[tokio::test]
    async fn every_answer_is_written_into_the_journal() {
        let service = FakeService::start(FakeAnswer::Stream(openai_events(&["PA", "SS"]))).await;
        let (_directory, db) = database();
        let local = Integration {
            url: format!("{}/v1/chat/completions", service.url),
            ca_certificate: service.ca_pem.clone(),
            ..integration()
        };
        let id = create_integration(&db, &local).unwrap();
        let answer = ask(&db, &question(id, "Is it so?")).await.unwrap();
        let entry = read_journal_entry(&db, answer.journal).unwrap();

        assert_eq!(answer.text, "PASS");
        assert_eq!(answer.ending, Ending::Complete);
        assert_eq!((entry.answer.as_str(), entry.ending, entry.work_unit), ("PASS", Ending::Complete, Some(7)));
        assert_eq!((entry.integration.as_str(), entry.model.as_str()), ("Ollama", "local-model"));
        assert!(entry.request.contains("Is it so?") && entry.request.contains("Be brief."));
    }

    #[tokio::test]
    async fn a_failure_is_written_with_its_explanation_and_never_the_key() {
        let service =
            FakeService::start(FakeAnswer::Json(401, r#"{"error": {"message": "bad sk-secret"}}"#.to_owned())).await;
        let (_directory, db) = database();
        let remote = Integration {
            url: format!("{}/v1/messages", service.url),
            ca_certificate: service.ca_pem.clone(),
            protocol: Protocol::AnthropicMessages,
            ..integration()
        };
        let id = create_integration(&db, &remote).unwrap();

        assert!(matches!(prepare(&db, &question(id, "Hi")), Err(AiError::NoKey)));

        create_api_key(&db, &ApiKey { integration: id, key: Zeroizing::new("sk-secret".to_owned()) }).unwrap();

        let answer = ask(&db, &question(id, "Hi")).await.unwrap();
        let entry = read_journal_entry(&db, answer.journal).unwrap();

        assert_eq!(entry.ending, Ending::Failed);
        assert!(matches!(answer.error, Some(AiError::KeyRejected(_))));
        assert!(entry.answer.contains("does not accept the API key") && !entry.answer.contains("sk-secret"));
        assert!(!entry.request.contains("sk-secret"));

        let named = FakeService::start(FakeAnswer::Stream(anthropic_events(&["ok"]))).await;
        let remote =
            Integration { url: format!("{}/v1/messages", named.url), ca_certificate: named.ca_pem.clone(), ..remote };

        // A new address: the key is given anew for it.
        update_integration(&db, &Integration { id, ..remote }).unwrap();
        assert!(matches!(prepare(&db, &question(id, "Hi")), Err(AiError::NoKey)));
        update_api_key(&db, &ApiKey { integration: id, key: Zeroizing::new("sk-secret".to_owned()) }).unwrap();

        let answer = ask(&db, &question(id, "Hi")).await.unwrap();

        assert_eq!((answer.text.as_str(), answer.model.as_str()), ("ok", "named-model"));
    }

    #[test]
    fn a_question_that_does_not_fit_or_asks_nothing_is_refused_before_it_is_sent() {
        let (_directory, db) = database();
        let small = Integration { context_size: 400, ..integration() };
        let id = create_integration(&db, &small).unwrap();

        // 400 tokens: 100 for the answer, 300 for the question (900
        // characters at most).
        assert!(prepare(&db, &question(id, &"x".repeat(880))).is_ok());
        assert!(matches!(prepare(&db, &question(id, &"x".repeat(1000))), Err(AiError::TooLong { room: 300, .. })));

        let mut nothing = question(id, " ");

        assert!(matches!(prepare(&db, &nothing), Err(AiError::NoQuestion)));

        nothing.messages.push(Message { speaker: Speaker::Ai, text: "an answer".to_owned(), time: 0 });
        assert!(matches!(prepare(&db, &nothing), Err(AiError::NoQuestion)));
    }
}
