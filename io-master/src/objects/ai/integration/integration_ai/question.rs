//! The first step of the way to the AI (user, 2026-10-29), under the lock
//! of the session: a question read with its integration and its key, made
//! into the body of the protocol of the integration, and checked that it
//! fits into the context of the model beside its answer.
//!
//! ```text
//! Question ──► prepare ──► Prepared ──► sending.rs: send ──► Sent ──► journal_ai.rs: record ──► Answer
//!               ├► read_integration, read_api_key       (the integration and its key)
//!               ├► check_address, turns, check_size      (refused, never shortened)
//!               └► protocol.rs: body                     (the body of the request)
//! ```
//!
//! The steps are apart because the database is not used while the answer
//! is awaited: the caller holds the lock of the session (and so the
//! database) only for `prepare` and `record`, so that the other commands
//! of the window are not held up by a long answer.

use zeroize::Zeroizing;

use crate::infrastructure::clock::now;
use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::ai::api_key::api_key_store::read_api_key;
use crate::objects::ai::chat::chat_model::{Message, Speaker};
use crate::objects::ai::integration::integration_ai::connection::Request;
use crate::objects::ai::integration::integration_ai::protocol::{body, needs_key};
use crate::objects::ai::integration::integration_ai::turns::{Turn, turns};
use crate::objects::ai::integration::integration_model::Integration;
use crate::objects::ai::integration::integration_rules::address::check_address;
use crate::objects::ai::integration::integration_store::read_integration;
use crate::objects::ai_error::AiError;

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
    pub(in crate::objects::ai) request: Request,
    /// The name of the integration asked ("Ollama"), for the journal.
    pub(in crate::objects::ai) integration_name: String,
    /// The model asked, for an answer that does not name its own.
    pub(in crate::objects::ai) model: String,
    pub(in crate::objects::ai) agent: Option<i64>,
    pub(in crate::objects::ai) work_unit: Option<i64>,
    pub(in crate::objects::ai) time: i64,
}

/// The longest answer asked of the integration: a fourth of its context,
/// at most [`MAX_ANSWER_TOKENS`], at least one token.
///
/// Called by: [`check_size`], [`prepare`].
fn answer_tokens(integration: &Integration) -> u32 {
    (integration.context_size / 4).clamp(1, MAX_ANSWER_TOKENS)
}

/// Refuses a question that does not fit into the context of the model
/// beside its answer (the estimate of [`CHARACTERS_PER_TOKEN`]).
///
/// Input: `integration`, the integration asked; `system`, the instructions;
/// `turns`, the conversation.
/// Output: nothing when it fits; `AiError::TooLong` with the estimate and the
/// room (`{ estimated: 9000, room: 6000 }`) when it does not.
///
/// Called by: [`prepare`].
/// Calls: [`answer_tokens`].
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
/// fits. Runs under the lock of the session.
///
/// Input: `question`, the question, its last message the user's.
/// Output: the question ready to be sent, with what its entry of the journal
/// needs; `AiError::TooManyMessages` over [`MAX_MESSAGES`],
/// `AiError::NoQuestion` with nothing to ask, `AiError::NoKey` when the
/// protocol needs a key the integration has not, `AiError::TooLong` when it
/// does not fit, an error for an address refused or a database failed.
///
/// Called by: `objects/ai/chat/chat_ai.rs`: `prepare_shortened`;
/// `objects/evaluation/work_unit/work_unit_ai/assessment.rs`: `prepared_assessment`;
/// `objects/ai/integration/integration_ai/check.rs`: `check_question`;
/// `objects/evaluation/work_unit/work_unit_ai/filling.rs`: `prepared_question`.
/// Calls: `objects/ai/integration/integration_store.rs`: `read_integration`; `objects/ai/api_key/api_key_store.rs`:
/// `read_api_key`; `objects/ai/integration/integration_rules/address.rs`: `check_address`;
/// `objects/ai/integration/integration_ai/turns.rs`: `turns`; [`check_size`], [`answer_tokens`];
/// `objects/ai/integration/integration_ai/protocol.rs`: `needs_key`, `body`; `infrastructure/clock.rs`: `now`.
pub fn prepare(db: &UserDatabase, question: &Question) -> Result<Prepared, AiError> {
    ensure!(
        question.messages.len() <= MAX_MESSAGES,
        AiError::TooManyMessages { count: question.messages.len(), max: MAX_MESSAGES }
    );

    let turns = turns(&question.messages);

    ensure!(matches!(turns.last(), Some(last) if last.speaker == Speaker::User), AiError::NoQuestion);

    let integration = read_integration(db, question.integration)?;

    check_address(&integration.url)?;

    let key = read_api_key(db, integration.id)?;

    ensure!(key.is_some() || !needs_key(integration.protocol), AiError::NoKey);
    check_size(&integration, &question.system, &turns)?;

    let time = now().map_err(|error| AiError::Refused(error.to_string()))?;
    let body = body(integration.protocol, &integration.model, &question.system, &turns, answer_tokens(&integration));
    let request = Request {
        url: integration.url,
        ca_certificate: integration.ca_certificate,
        protocol: integration.protocol,
        key: key.map(|key| Zeroizing::new(key.key.as_str().to_owned())),
        body: body.to_string(),
    };

    Ok(Prepared {
        request,
        integration_name: integration.name,
        model: integration.model,
        agent: question.agent,
        work_unit: question.work_unit,
        time,
    })
}

#[cfg(test)]
mod tests {
    //! Tests of the question prepared: its size and its last message.

    use super::*;

    use crate::objects::ai::integration::integration_store::create_integration;
    use crate::test_support::database::{database, integration};

    /// A question without an agent to the integration `integration`.
    fn question(integration: i64, text: &str) -> Question {
        Question {
            integration,
            agent: None,
            work_unit: Some(7),
            system: "Be brief.".to_owned(),
            messages: vec![Message { speaker: Speaker::User, text: text.to_owned(), time: 0 }],
        }
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
        let mut many = question(id, "x");

        many.messages = vec![Message { speaker: Speaker::User, text: "x".to_owned(), time: 0 }; MAX_MESSAGES + 1];
        assert!(matches!(
            prepare(&db, &many),
            Err(AiError::TooManyMessages { count, max: MAX_MESSAGES }) if count == MAX_MESSAGES + 1
        ));
    }
}
