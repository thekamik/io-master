//! The chat (user, 2026-10-30): a question of a chat prepared with as many
//! of the latest messages of the chat as fit into the context of the model.
//!
//! **A long chat is shortened** (a chat is no part of an evaluation): when
//! the question does not fit, its oldest messages are left out of it, never
//! the instructions, the knowledge or the question. The chat keeps them all,
//! and the journal of the agent keeps the question as it was sent.
//!
//! ```text
//! Question ──► prepare ──► too long? ──► leave_oldest_out ──► prepare ... ──► (Prepared, messages left out)
//! ```

use crate::ai::error::AiError;
use crate::ai::models::ai::integration::question::{MAX_MESSAGES, Prepared, Question, prepare};
use crate::db::storage::user_database::UserDatabase;
use crate::model::ai::chat::Speaker;

/// Leaves the oldest message of the conversation of `question` out, and an
/// answer that would then come first, so that it starts with the user.
///
/// Called by: [`prepare_shortened`].
fn leave_oldest_out(question: &mut Question) {
    question.messages.remove(0);

    if matches!(question.messages.first(), Some(first) if first.speaker == Speaker::Ai) {
        question.messages.remove(0);
    }
}

/// Prepares the question `question` of a chat, its oldest messages left out
/// until it fits into the context of the model of its integration.
///
/// Input:
/// - `question`: the question of a chat (`ai/models/ai/agent.rs`:
///   `agent_question`), its last message the question of the user.
///
/// Output: the question prepared and how many messages were left out of it
/// (`(prepared, 4)`). An error when it does not fit even with the question
/// of the user alone, or `prepare` refuses it for another reason.
///
/// Called by: `panels/logic/chat/chats.rs`: `prepared`.
/// Calls: `integration/question.rs`: `prepare`; [`leave_oldest_out`].
pub fn prepare_shortened(db: &UserDatabase, mut question: Question) -> Result<(Prepared, usize), AiError> {
    ensure!(!question.messages.is_empty(), AiError::NoQuestion);

    let all = question.messages.len();

    // Each turn leaves at least one message out, so the loop ends within the
    // messages of one question.
    for _ in 0..=MAX_MESSAGES {
        match prepare(db, &question) {
            Ok(prepared) => return Ok((prepared, all - question.messages.len())),
            Err(AiError::TooLong { .. } | AiError::TooManyMessages { .. }) if question.messages.len() > 1 => {
                leave_oldest_out(&mut question)
            }
            Err(error) => return Err(error),
        }
    }

    Err(AiError::TooManyMessages { count: all, max: MAX_MESSAGES })
}

#[cfg(test)]
mod tests {
    //! Tests of a question of a chat shortened until it fits.

    use super::*;

    use crate::db::models::ai::integration::create_integration;
    use crate::model::ai::chat::Message;
    use crate::model::ai::integration::Integration;
    use crate::test_support::database::{database, integration};

    /// A message of `speaker` of `length` characters.
    fn message(speaker: Speaker, length: usize) -> Message {
        Message { speaker, text: "x".repeat(length), time: 0 }
    }

    #[test]
    fn the_oldest_messages_are_left_out_until_the_question_fits() {
        let (_directory, db) = database();
        let small = create_integration(&db, &Integration { context_size: 400, ..integration() }).unwrap();
        let question =
            |messages| Question { integration: small, agent: None, work_unit: None, system: String::new(), messages };
        // 300 tokens for the question, about 900 characters: the last
        // question fits, the first question and its answer with it do not.
        let messages = vec![message(Speaker::User, 400), message(Speaker::Ai, 400), message(Speaker::User, 300)];
        let (_, left_out) = prepare_shortened(&db, question(messages)).unwrap();

        assert_eq!(left_out, 2);
        assert!(matches!(
            prepare_shortened(&db, question(vec![message(Speaker::User, 1000)])),
            Err(AiError::TooLong { .. })
        ));
    }
}
