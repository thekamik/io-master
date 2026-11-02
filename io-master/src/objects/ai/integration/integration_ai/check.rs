//! The question that checks an integration (the settings of the AI): one
//! short message, no instructions, no agent, so it is not kept in a journal
//! (`objects/ai/journal/journal_ai.rs`: `record`).

use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::ai::chat::chat_model::{Message, Speaker};
use crate::objects::ai::integration::integration_ai::question::{Prepared, Question, prepare};
use crate::objects::ai_error::AiError;

/// The question that checks an integration, and the longest answer it asks
/// for: a word, with room for a model that thinks before it answers, small
/// enough for the smallest context.
const CHECK_QUESTION: &str = "Reply with the word OK.";
const CHECK_ANSWER_TOKENS: u32 = 256;

/// The question that checks the integration `id`, prepared.
///
/// Input: `id`, the integration to check.
///
/// Output: the question, prepared to be sent; an error of `prepare` (the
/// integration missing, its address refused, no key where its protocol needs
/// one).
///
/// Called by: `panels/views/settings/integrations.rs`: `integration_check`.
/// Calls: `objects/ai/integration/integration_ai/question.rs`: `prepare`.
pub fn check_question(db: &UserDatabase, id: i64) -> Result<Prepared, AiError> {
    let question = Question {
        integration: id,
        agent: None,
        work_unit: None,
        system: String::new(),
        messages: vec![Message { speaker: Speaker::User, text: CHECK_QUESTION.to_owned(), time: 0 }],
        max_answer_tokens: CHECK_ANSWER_TOKENS,
    };

    prepare(db, &question)
}
