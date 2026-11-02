//! The rules of a chat (`Chat`, user 2026-10-30): the user chooses an agent
//! and talks with the model of its integration. A chat is never removed;
//! its title may be changed. The question of a chat with the knowledge of
//! its agent, and its answer written, are a part of the AI
//! (`objects/ai/chat/chat_ai.rs`).
//!
//! ```text
//! new_chat(title, agent) ──► create_chat (no messages yet)
//! rename_chat            ──► update_chat (the title only)
//! add_question           ──► the question written into the chat, once objects/ai/chat/chat_ai.rs: prepared took it
//! ```

use crate::infrastructure::clock::now;
use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::ai::agent::agent_store::read_agent;
use crate::objects::ai::chat::chat_model::{Chat, Message, Speaker};
use crate::objects::ai::chat::chat_store::{create_chat, create_message, read_chat, update_chat};
use crate::objects::error::DomainError;
use crate::objects::text::checked_text;

/// The longest title of a chat, in characters.
const MAX_TITLE: usize = 200;

/// The longest question of the user, in characters.
pub const MAX_QUESTION: usize = 20_000;

/// Makes a new chat with the agent `agent`, without messages.
///
/// Input: `title`, the title typed ("The ST of the router"); `agent`, the id of
/// a stored agent.
/// Output: the id of the new chat (`5`); `DomainError::Refused` for an empty
/// title or one over [`MAX_TITLE`], an error of the database for an agent
/// missing.
///
/// Called by: `panels/views/chat/chats.rs`: `chat_create`.
/// Calls: `objects/text.rs`: `checked_text`; `objects/ai/agent/agent_store.rs`: `read_agent`;
/// `objects/ai/chat/chat_store.rs`: `create_chat`.
pub fn new_chat(db: &UserDatabase, title: &str, agent: i64) -> Result<i64, DomainError> {
    let title = checked_text(title, "the title of the chat", MAX_TITLE)?;

    read_agent(db, agent)?;

    Ok(create_chat(db, &Chat { id: 0, title, agent: Some(agent), work_unit: None, messages: Vec::new() })?)
}

/// Writes the title `title` of the chat `chat`; its agent and its messages
/// stay as they were.
///
/// Input: `chat`, the id of a stored chat; `title`, the new title typed.
/// Output: nothing; `DomainError::Refused` for a title refused, an error of
/// the database for a chat missing.
///
/// Called by: `panels/views/chat/chats.rs`: `chat_rename`.
/// Calls: `objects/ai/chat/chat_store.rs`: `read_chat`, `update_chat`; `objects/text.rs`:
/// `checked_text`.
pub fn rename_chat(db: &UserDatabase, chat: i64, title: &str) -> Result<(), DomainError> {
    let mut stored = read_chat(db, chat)?;

    stored.title = checked_text(title, "the title of the chat", MAX_TITLE)?;

    Ok(update_chat(db, &stored)?)
}

/// The question trimmed; refused when it is empty or over
/// [`MAX_QUESTION`].
///
/// Called by: [`add_question`]; `objects/ai/chat/chat_ai.rs`: `chat_question`.
/// Calls: `objects/text.rs`: `checked_text`.
pub fn checked_question(text: &str) -> Result<String, DomainError> {
    checked_text(text, "the question", MAX_QUESTION)
}

/// Writes the question `text` of the user as the last message of the chat
/// `chat`.
///
/// Input: `chat`, the id of a stored chat; `text`, the question as typed.
/// Output: nothing; `DomainError::Refused` for a question refused or a clock
/// before 1970, an error of the database.
///
/// Called by: `panels/views/chat/chats.rs`: `prepared_question`, once the
/// question is prepared.
/// Calls: [`checked_question`]; `infrastructure/clock.rs`: `now`; `objects/ai/chat/chat_store.rs`:
/// `create_message`.
pub fn add_question(db: &UserDatabase, chat: i64, text: &str) -> Result<(), DomainError> {
    let message = Message {
        speaker: Speaker::User,
        text: checked_question(text)?,
        time: now().map_err(|error| DomainError::Refused(error.to_string()))?,
    };

    Ok(create_message(db, chat, &message)?)
}

#[cfg(test)]
mod tests {
    //! Tests of the chats: made and renamed, a question written.

    use super::*;

    use crate::objects::ai::agent::agent_store::create_agent;
    use crate::objects::ai::integration::integration_store::create_integration;
    use crate::test_support::database::{agent, database, integration};

    #[test]
    fn a_chat_is_made_with_an_agent_and_renamed() {
        let (_directory, db) = database();
        let local = create_integration(&db, &integration()).unwrap();
        let evaluator = create_agent(&db, &agent(local)).unwrap();

        assert!(matches!(new_chat(&db, "  ", evaluator), Err(DomainError::Refused(_))));
        assert!(matches!(new_chat(&db, "ST", 999), Err(DomainError::Db(_))));

        let chat = new_chat(&db, " The ST of the router ", evaluator).unwrap();

        rename_chat(&db, chat, "The TOE").unwrap();
        add_question(&db, chat, " What is the TOE? ").unwrap();
        assert!(matches!(add_question(&db, chat, " "), Err(DomainError::Refused(_))));

        let stored = read_chat(&db, chat).unwrap();

        assert_eq!((stored.title.as_str(), stored.agent), ("The TOE", Some(evaluator)));
        assert_eq!(stored.messages[0].text, "What is the TOE?");
        assert!(matches!(rename_chat(&db, chat, &"x".repeat(MAX_TITLE + 1)), Err(DomainError::Refused(_))));
    }
}
