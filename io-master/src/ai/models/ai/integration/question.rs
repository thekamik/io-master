//! The first step of the way to the AI (user, 2026-10-29), under the lock
//! of the session: a question read with its integration and its key, made
//! into the body of the protocol of the integration, and checked that it
//! fits into the context of the model beside its answer.
//!
//! ```text
//! Question ──► prepare ──► Prepared ──► sending.rs: send ──► Sent ──► ../journal.rs: record ──► Answer
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

use crate::ai::error::AiError;
use crate::ai::models::ai::integration::address::check_address;
use crate::ai::models::ai::integration::connection::Request;
use crate::ai::models::ai::integration::protocol::{body, needs_key};
use crate::ai::models::ai::integration::turns::{Turn, turns};
use crate::clock::now;
use crate::db::models::ai::api_key::read_api_key;
use crate::db::models::ai::integration::read_integration;
use crate::db::storage::user_database::UserDatabase;
use crate::model::ai::chat::{Message, Speaker};
use crate::model::ai::integration::Integration;

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
    pub(in crate::ai::models::ai) request: Request,
    /// The name of the integration asked ("Ollama"), for the journal.
    pub(in crate::ai::models::ai) integration_name: String,
    /// The model asked, for an answer that does not name its own.
    pub(in crate::ai::models::ai) model: String,
    pub(in crate::ai::models::ai) agent: Option<i64>,
    pub(in crate::ai::models::ai) work_unit: Option<i64>,
    pub(in crate::ai::models::ai) time: i64,
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
/// Called by: `ai/models/ai/chat.rs`: `prepare_shortened`;
/// `ai/models/evaluation/work_unit/assessment.rs`: `prepared_assessment`;
/// `panels/logic/settings/integrations.rs`: `check_question`;
/// `ai/models/evaluation/work_unit/filling.rs`: `prepared_question`; under the
/// lock of the session.
/// Calls: `db/models/ai/integration.rs`: `read_integration`;
/// `db/models/ai/api_key.rs`: `read_api_key`; `address.rs`:
/// `check_address`; `turns.rs`: `turns`; [`check_size`];
/// `protocol.rs`: `needs_key`, `body`; `clock.rs`: `now`.
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

    use crate::db::models::ai::integration::create_integration;
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
