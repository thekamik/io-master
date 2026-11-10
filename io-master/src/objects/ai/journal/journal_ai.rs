//! The third step of the way to the AI: the answer written into the journal
//! of its agent, and only then given out (user, 2026-10-29), so that no
//! caller can skip the journal. A question without an agent (the check of
//! an integration) is not kept: it is no part of an evaluation.
//!
//! ```text
//! integration_ai/question.rs: prepare ──► Prepared ──► integration_ai/sending.rs: send ──► Sent ──► record ──► Answer
//!                                                                                             └► create_journal_entry (with an agent)
//! ```
//!
//! The caller holds the lock of the session for it, as for `prepare`.

use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::ai::integration::integration_ai::question::{Prepared, shortened_warning};
use crate::objects::ai::integration::integration_ai::sending::Sent;
use crate::objects::ai::journal::journal_model::{Ending, JournalEntry};
use crate::objects::ai::journal::journal_store::create_journal_entry;
use crate::objects::ai_error::AiError;

/// The answer, written into the journal of its agent.
#[derive(Debug)]
pub struct Answer {
    /// The id of its entry in the journal; `None` for a question without an
    /// agent, which is not kept.
    pub journal: Option<i64>,
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
    /// The warning when the service counted far fewer tokens than the
    /// question was estimated at: it may have shortened the question in
    /// silence (`integration_ai/question.rs`: `shortened_warning`).
    pub shortened: Option<String>,
}

/// Why a work of the AI does not take an answer that ended as `ending`: a
/// cut answer may hold a whole JSON object with half an analysis, so only an
/// answer the model ended itself is taken (user, 2026-11-02).
///
/// Input: `ending`, how the answer ended (`Answer::ending`).
/// Output: the reason ("it was cut at its largest length"); `None` for a
/// complete answer.
///
/// Called by: `objects/evaluation/work_unit/work_unit_ai/assessment.rs`: `written`;
/// `objects/evaluation/work_unit/work_unit_ai/filling.rs`: `written`.
pub fn why_not_taken(ending: Ending) -> Option<&'static str> {
    match ending {
        Ending::Complete => None,
        Ending::Cut => Some("it was cut at its largest length"),
        Ending::Refused => Some("the model declined to answer"),
        Ending::Stopped => Some("it was stopped"),
        Ending::Failed => Some("it failed"),
    }
}

/// Writes the question of the agent `agent` and its answer into the
/// journal of the agent. On a failure the explanation is written after the
/// text of the answer, so that the journal tells why it failed.
///
/// Input: `prepared`, the question as it was sent; `agent`, the id of the
/// agent that asked; `answer`, the answer as [`record`] read it.
/// Output: the id of the entry (`41`); an error of the database.
///
/// Called by: [`record`].
/// Calls: `objects/ai/journal/journal_store.rs`: `create_journal_entry`.
fn write_entry(db: &UserDatabase, prepared: Prepared, agent: i64, answer: &Answer) -> Result<i64, AiError> {
    let mut written = answer.text.clone();

    if let Some(error) = &answer.error {
        written.push_str("\n\n[the question failed: ");
        written.push_str(&error.to_string());
        written.push(']');
    }

    let entry = JournalEntry {
        id: 0,
        time: prepared.time,
        integration: prepared.integration_name,
        model: answer.model.clone(),
        agent,
        work_unit: prepared.work_unit,
        request: prepared.request.body,
        answer: written,
        ending: answer.ending,
        input_tokens: answer.input_tokens,
        output_tokens: answer.output_tokens,
    };

    Ok(create_journal_entry(db, &entry)?)
}

/// Gives the answer out, written first into the journal of its agent; the
/// answer to a question without an agent is not kept. Runs under the lock
/// of the session.
///
/// Input: `prepared`, the question as it was sent; `sent`, its answer as it
/// came.
/// Output: the answer with how it ended and the id of its entry
/// (`Answer { journal: Some(41), ending: Complete, .. }`); an error of the
/// database when the entry cannot be written.
///
/// Called by: `panels/views/settings/integrations.rs`: `integration_check`;
/// `objects/evaluation/work_unit/work_unit_ai/filling.rs`: `written`;
/// `objects/evaluation/work_unit/work_unit_ai/assessment.rs`: `written`;
/// `panels/views/chat/chats.rs`: `written`.
/// Calls: `objects/ai/integration/integration_ai/question.rs`: `shortened_warning`; [`write_entry`].
pub fn record(db: &UserDatabase, prepared: Prepared, sent: Sent) -> Result<Answer, AiError> {
    let Sent(received) = sent;
    let reading = received.reading;
    let ending = match (&received.error, received.stopped, reading.ending) {
        (Some(_), _, _) => Ending::Failed,
        (None, true, _) => Ending::Stopped,
        (None, false, ending) => ending.unwrap_or(Ending::Complete),
    };
    let mut answer = Answer {
        journal: None,
        text: reading.text,
        model: reading.model.unwrap_or_else(|| prepared.model.clone()),
        ending,
        error: received.error,
        input_tokens: reading.input_tokens,
        output_tokens: reading.output_tokens,
        shortened: shortened_warning(&prepared, reading.input_tokens),
    };

    if let Some(agent) = prepared.agent {
        answer.journal = Some(write_entry(db, prepared, agent, &answer)?);
    }

    Ok(answer)
}

#[cfg(test)]
mod tests {
    //! Tests of the whole way to the AI: a question prepared, sent to the
    //! fake service and written into the journal.

    use super::*;

    use tokio::sync::mpsc;
    use zeroize::Zeroizing;

    use crate::objects::ai::agent::agent_store::create_agent;
    use crate::objects::ai::api_key::api_key_model::ApiKey;
    use crate::objects::ai::api_key::api_key_store::update_api_key;
    use crate::objects::ai::chat::chat_model::{Message, Speaker};
    use crate::objects::ai::integration::integration_ai::question::{Question, prepare};
    use crate::objects::ai::integration::integration_ai::sending::send;
    use crate::objects::ai::integration::integration_model::{Integration, Protocol};
    use crate::objects::ai::integration::integration_store::{create_integration, update_integration};
    use crate::objects::ai::journal::journal_store::{read_journal_entries, read_journal_entry};
    use crate::test_support::ai_service::{FakeAnswer, FakeService, anthropic_events, openai_events};
    use crate::test_support::database::{agent, database, integration};

    /// A question of the agent `agent` (none with `None`) to the
    /// integration `integration`.
    fn question(integration: i64, agent: Option<i64>, text: &str) -> Question {
        Question {
            integration,
            agent,
            work_unit: Some(7),
            system: "Be brief.".to_owned(),
            messages: vec![Message { speaker: Speaker::User, text: text.to_owned(), time: 0 }],
            max_answer_tokens: 16_000,
        }
    }

    /// Asks through the three steps, the pieces drained.
    async fn ask(db: &UserDatabase, question: &Question) -> Result<Answer, AiError> {
        let prepared = prepare(db, question)?;
        let (sender, mut receiver) = mpsc::channel(16);
        let draining = tokio::spawn(async move { while receiver.recv().await.is_some() {} });
        let sent = send(&prepared, &sender).await;

        drop(sender);
        draining.await.unwrap();
        record(db, prepared, sent)
    }

    #[tokio::test]
    async fn the_answer_of_an_agent_is_written_into_its_journal_and_one_without_an_agent_is_not() {
        let service = FakeService::start(FakeAnswer::Stream(openai_events(&["PA", "SS"]))).await;
        let (_directory, db) = database();
        let local = Integration {
            url: format!("{}/v1/chat/completions", service.url),
            ca_certificate: service.ca_pem.clone(),
            ..integration()
        };
        let id = create_integration(&db, &local).unwrap();
        let asking = create_agent(&db, &agent(id)).unwrap();
        let answer = ask(&db, &question(id, Some(asking), "Is it so?")).await.unwrap();
        let entry = read_journal_entry(&db, answer.journal.unwrap()).unwrap();

        assert_eq!(answer.text, "PASS");
        assert_eq!(answer.ending, Ending::Complete);
        assert_eq!((entry.answer.as_str(), entry.ending, entry.work_unit), ("PASS", Ending::Complete, Some(7)));
        assert_eq!((entry.integration.as_str(), entry.model.as_str(), entry.agent), ("Ollama", "local-model", asking));
        assert!(entry.request.contains("Is it so?") && entry.request.contains("Be brief."));

        let unkept = ask(&db, &question(id, None, "A check")).await.unwrap();

        assert_eq!((unkept.text.as_str(), unkept.journal), ("PASS", None));
        assert_eq!(read_journal_entries(&db, asking, None).unwrap().len(), 1);
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
        let asking = Some(create_agent(&db, &agent(id)).unwrap());

        assert!(matches!(prepare(&db, &question(id, asking, "Hi")), Err(AiError::NoKey)));

        update_api_key(&db, &ApiKey { integration: id, key: Zeroizing::new("sk-secret".to_owned()) }).unwrap();

        let answer = ask(&db, &question(id, asking, "Hi")).await.unwrap();
        let entry = read_journal_entry(&db, answer.journal.unwrap()).unwrap();

        assert_eq!(entry.ending, Ending::Failed);
        assert!(matches!(answer.error, Some(AiError::KeyRejected(_))));
        assert!(entry.answer.contains("does not accept the API key") && !entry.answer.contains("sk-secret"));
        assert!(!entry.request.contains("sk-secret"));

        let named = FakeService::start(FakeAnswer::Stream(anthropic_events(&["ok"]))).await;
        let remote =
            Integration { url: format!("{}/v1/messages", named.url), ca_certificate: named.ca_pem.clone(), ..remote };

        // A new address: the key is given anew for it.
        update_integration(&db, &Integration { id, ..remote }).unwrap();
        assert!(matches!(prepare(&db, &question(id, asking, "Hi")), Err(AiError::NoKey)));
        update_api_key(&db, &ApiKey { integration: id, key: Zeroizing::new("sk-secret".to_owned()) }).unwrap();

        let answer = ask(&db, &question(id, asking, "Hi")).await.unwrap();

        assert_eq!((answer.text.as_str(), answer.model.as_str()), ("ok", "named-model"));
    }
}
