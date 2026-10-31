//! The chats (user, 2026-10-30): the user chooses an agent and talks with
//! the model of its integration, which gets the knowledge of the agent
//! with every question. A chat is never removed; its title may be changed.
//!
//! ```text
//! new_chat(title, agent) ──► create_chat (no messages yet)
//!
//! a question of the user:
//!   chat_question ──► the instructions of the agent
//!                     + the knowledge of the agent that fits the question (RAG)
//!                     + every message of the chat and the question ──► Question (ai/ask.rs)
//!   prepared      ──► ask::prepare; the oldest messages left out until the question fits
//!   add_question  ──► the question written into the chat, once ask::prepare took it
//!   add_answer    ──► the answer written into the chat; a stopped or cut one with a mark,
//!                     a failed one not at all (its reason is in the journal of the agent)
//! ```
//!
//! **The knowledge** is looked for with the question and the question of the
//! user before it, so that a short question that goes on with the earlier
//! one ("and in the ST?") still finds what the conversation is about.
//!
//! **A long chat is shortened** (user, 2026-10-30: a chat is no part of an
//! evaluation): when the question does not fit into the context of the
//! model, its oldest messages are left out of it, never the instructions,
//! the knowledge or the question; the page is told how many. The chat keeps
//! them all, and the journal of the agent keeps the question as it was sent.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::ai::ask::{Answer, Prepared, Question, prepare};
use crate::ai::error::AiError;
use crate::db::models::ai::agent::read_agent;
use crate::db::models::ai::chat::{MAX_MESSAGES, create_chat, create_message, read_chat, update_chat};
use crate::db::storage::user_database::UserDatabase;
use crate::model::ai::chat::{Chat, Message, Speaker};
use crate::model::ai::journal::Ending;
use crate::panels::logic::agents::agent::agent_knowledge;
use crate::panels::logic::chat::error::ChatError;

/// The longest title of a chat, in characters.
const MAX_TITLE: usize = 200;

/// The longest question of the user, in characters.
pub const MAX_QUESTION: usize = 20_000;

/// The marks written after an answer that did not end by itself.
const STOPPED: &str = "[stopped]";
const CUT: &str = "[cut]";
const REFUSED: &str = "[refused]";

/// The time now, in seconds since 1970 (UTC).
///
/// Called by: [`add_question`], [`add_answer`].
fn now() -> Result<i64, ChatError> {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| ChatError::Refused(format!("the clock: {error}")))?
        .as_secs();

    i64::try_from(seconds).map_err(|error| ChatError::Refused(format!("the clock: {error}")))
}

/// The title trimmed; refused when it is empty or over [`MAX_TITLE`].
///
/// Called by: [`new_chat`], [`rename_chat`].
fn checked_title(title: &str) -> Result<String, ChatError> {
    let title = title.trim();

    ensure!(!title.is_empty(), ChatError::Refused("a chat without a title".to_owned()));
    ensure!(title.chars().count() <= MAX_TITLE, ChatError::Refused(format!("a title over {MAX_TITLE} characters")));

    Ok(title.to_owned())
}

/// Makes a new chat with the agent `agent`, without messages; gives its id.
///
/// Called by: `panels/views/chat/chats.rs` (`chat_create`).
/// Calls: [`checked_title`], `read_agent`, `create_chat`.
pub fn new_chat(db: &UserDatabase, title: &str, agent: i64) -> Result<i64, ChatError> {
    let title = checked_title(title)?;

    read_agent(db, agent)?;

    Ok(create_chat(db, &Chat { id: 0, title, agent: Some(agent), work_unit: None, messages: Vec::new() })?)
}

/// Writes the title `title` of the chat `chat`; its agent and its messages
/// stay as they were.
///
/// Called by: `panels/views/chat/chats.rs` (`chat_rename`).
/// Calls: `read_chat`, [`checked_title`], `update_chat`.
pub fn rename_chat(db: &UserDatabase, chat: i64, title: &str) -> Result<(), ChatError> {
    let mut stored = read_chat(db, chat)?;

    stored.title = checked_title(title)?;

    Ok(update_chat(db, &stored)?)
}

/// The question trimmed; refused when it is empty or over
/// [`MAX_QUESTION`].
///
/// Called by: [`chat_question`], [`add_question`].
fn checked_question(text: &str) -> Result<String, ChatError> {
    let text = text.trim();

    ensure!(!text.is_empty(), ChatError::Refused("an empty question".to_owned()));
    ensure!(
        text.chars().count() <= MAX_QUESTION,
        ChatError::Refused(format!("a question over {MAX_QUESTION} characters"))
    );

    Ok(text.to_owned())
}

/// What the knowledge of the agent is looked for with: the question and
/// the last question of the user before it.
///
/// Called by: [`chat_question`].
fn search_text(messages: &[Message], question: &str) -> String {
    for message in messages.iter().rev() {
        if message.speaker == Speaker::User {
            return format!("{} {question}", message.text);
        }
    }

    question.to_owned()
}

/// The question `text` in the chat `chat`, as the integration of its agent
/// gets it: the instructions and the knowledge of the agent, the messages
/// of the chat and the question. Nothing is written; refused when the chat
/// has no agent or no room for the question and its answer.
///
/// Called by: [`prepared`].
/// Calls: `read_chat`, [`checked_question`], `read_agent`, [`search_text`],
/// `agent_knowledge`.
fn chat_question(db: &UserDatabase, chat: i64, text: &str) -> Result<Question, ChatError> {
    let stored = read_chat(db, chat)?;
    let question = checked_question(text)?;
    let Some(agent_id) = stored.agent else {
        return Err(ChatError::Refused("the chat has no agent".to_owned()));
    };

    ensure!(
        stored.messages.len() + 2 <= MAX_MESSAGES,
        ChatError::Refused(format!("the chat is full ({MAX_MESSAGES} messages): begin a new one"))
    );

    let agent = read_agent(db, agent_id)?;
    let knowledge = agent_knowledge(db, agent_id, &search_text(&stored.messages, &question))?;
    let mut system = agent.instructions;

    if !knowledge.is_empty() {
        system.push_str("\n\nYour knowledge for this question:\n\n");
        system.push_str(&knowledge.join("\n\n---\n\n"));
    }

    let mut messages = stored.messages;

    messages.push(Message { speaker: Speaker::User, text: question, time: 0 });

    Ok(Question { integration: agent.integration, agent: Some(agent_id), work_unit: None, system, messages })
}

/// Leaves the oldest message of the conversation of `question` out, and an
/// answer that would then come first, so that it starts with the user.
///
/// Called by: [`prepared`].
fn leave_oldest_out(question: &mut Question) {
    question.messages.remove(0);

    if matches!(question.messages.first(), Some(first) if first.speaker == Speaker::Ai) {
        question.messages.remove(0);
    }
}

/// The question `text` in the chat `chat`, prepared for the integration of
/// its agent, with as many of the latest messages of the chat as fit into
/// the context of the model; gives also how many messages were left out.
/// Refused when the question does not fit even alone.
///
/// Called by: `panels/views/chat/chats.rs` (`chat_ask`).
/// Calls: [`chat_question`], `ask::prepare`, [`leave_oldest_out`].
pub fn prepared(db: &UserDatabase, chat: i64, text: &str) -> Result<(Prepared, usize), ChatError> {
    let mut question = chat_question(db, chat, text)?;
    let all = question.messages.len();

    for _ in 0..=MAX_MESSAGES {
        match prepare(db, &question) {
            Ok(prepared) => return Ok((prepared, all - question.messages.len())),
            Err(AiError::TooLong { .. }) if question.messages.len() > 1 => leave_oldest_out(&mut question),
            Err(error) => return Err(ChatError::Ai(error)),
        }
    }

    Err(ChatError::Refused(format!("more than {MAX_MESSAGES} messages left out of a question")))
}

/// Writes the question `text` of the user as the last message of the chat
/// `chat`.
///
/// Called by: `panels/views/chat/chats.rs` (`chat_ask`, once the question is
/// prepared).
/// Calls: [`checked_question`], [`now`], `create_message`.
pub fn add_question(db: &UserDatabase, chat: i64, text: &str) -> Result<(), ChatError> {
    let message = Message { speaker: Speaker::User, text: checked_question(text)?, time: now()? };

    Ok(create_message(db, chat, &message)?)
}

/// The text of the answer `answer` as the chat keeps it: an answer that did
/// not end by itself gets its mark; a failed one, or one without text that
/// ended by itself, is not kept.
///
/// Called by: [`add_answer`].
fn kept_text(answer: &Answer) -> Option<String> {
    let text = answer.text.trim();
    let mark = match answer.ending {
        Ending::Complete => None,
        Ending::Stopped => Some(STOPPED),
        Ending::Cut => Some(CUT),
        Ending::Refused => Some(REFUSED),
        Ending::Failed => return None,
    };

    match (text.is_empty(), mark) {
        (true, None) => None,
        (true, Some(mark)) => Some(mark.to_owned()),
        (false, None) => Some(text.to_owned()),
        (false, Some(mark)) => Some(format!("{text}\n\n{mark}")),
    }
}

/// Writes the answer `answer` as the last message of the chat `chat`, as
/// [`kept_text`] keeps it; gives the message written, `None` when none was.
///
/// Called by: `panels/views/chat/chats.rs` (`chat_ask`, after `ask::record`).
/// Calls: [`kept_text`], [`now`], `create_message`.
pub fn add_answer(db: &UserDatabase, chat: i64, answer: &Answer) -> Result<Option<Message>, ChatError> {
    let Some(text) = kept_text(answer) else {
        return Ok(None);
    };
    let message = Message { speaker: Speaker::Ai, text, time: now()? };

    create_message(db, chat, &message)?;

    Ok(Some(message))
}

#[cfg(test)]
mod tests {
    //! Tests of the chats: made and renamed, the question of an agent with
    //! its knowledge and the conversation, the answers kept with their marks.

    use super::*;

    use crate::db::models::ai::agent::create_agent;
    use crate::db::models::ai::integration::create_integration;
    use crate::db::models::ai::rag::{create_knowledge_base, create_source};
    use crate::model::ai::integration::Integration;
    use crate::model::ai::rag::{KnowledgeBase, Source};
    use crate::test_support::database::{agent, database, integration};

    fn answer(text: &str, ending: Ending) -> Answer {
        Answer {
            journal: Some(1),
            text: text.to_owned(),
            model: "llama3.1".to_owned(),
            ending,
            error: None,
            input_tokens: 0,
            output_tokens: 0,
        }
    }

    #[test]
    fn a_chat_is_made_with_an_agent_and_renamed() {
        let (_directory, db) = database();
        let local = create_integration(&db, &integration()).unwrap();
        let evaluator = create_agent(&db, &agent(local)).unwrap();

        assert!(matches!(new_chat(&db, "  ", evaluator), Err(ChatError::Refused(_))));
        assert!(matches!(new_chat(&db, "ST", 999), Err(ChatError::Db(_))));

        let chat = new_chat(&db, " The ST of the router ", evaluator).unwrap();

        rename_chat(&db, chat, "The TOE").unwrap();

        let stored = read_chat(&db, chat).unwrap();

        assert_eq!((stored.title.as_str(), stored.agent), ("The TOE", Some(evaluator)));
        assert!(matches!(rename_chat(&db, chat, &"x".repeat(MAX_TITLE + 1)), Err(ChatError::Refused(_))));
    }

    #[test]
    fn the_question_carries_the_instructions_the_knowledge_and_the_conversation() {
        let (_directory, db) = database();
        let local = create_integration(&db, &integration()).unwrap();
        let base =
            create_knowledge_base(&db, &KnowledgeBase { id: 0, name: "CC".to_owned(), description: String::new() })
                .unwrap();
        let source =
            Source { id: 0, knowledge_base: base, name: "notes".to_owned(), text: "The TSF is ...".to_owned() };

        create_source(&db, &source).unwrap();

        let mut with_knowledge = agent(local);

        with_knowledge.knowledge_bases = vec![base];

        let evaluator = create_agent(&db, &with_knowledge).unwrap();
        let chat = new_chat(&db, "TSF", evaluator).unwrap();

        add_question(&db, chat, "What is the TSF?").unwrap();
        add_answer(&db, chat, &answer("The security functions.", Ending::Complete)).unwrap();

        // The second question names no word of the knowledge; the first does.
        let question = chat_question(&db, chat, " And its interfaces? ").unwrap();

        assert_eq!((question.integration, question.agent, question.work_unit), (local, Some(evaluator), None));
        assert!(question.system.starts_with("You assess work units of the CEM."));
        assert!(question.system.contains("The TSF is ..."));
        assert_eq!(question.messages.len(), 3);
        assert_eq!(question.messages[2].text, "And its interfaces?");
        assert!(matches!(chat_question(&db, chat, " "), Err(ChatError::Refused(_))));
    }

    #[test]
    fn the_oldest_messages_are_left_out_of_a_question_that_does_not_fit() {
        let (_directory, db) = database();
        // 400 tokens: 100 for the answer, 300 for the question (900
        // characters at most, the instructions of the agent among them).
        let small = create_integration(&db, &Integration { context_size: 400, ..integration() }).unwrap();
        let chat = new_chat(&db, "x", create_agent(&db, &agent(small)).unwrap()).unwrap();

        for _ in 0..3 {
            add_question(&db, chat, &"q".repeat(200)).unwrap();
            add_answer(&db, chat, &answer(&"a".repeat(200), Ending::Complete)).unwrap();
        }

        let (_, left_out) = prepared(&db, chat, "And now?").unwrap();

        // Six messages of 200 characters, the question and the instructions
        // (1241 characters): the oldest pair is left out, 841 stay.
        assert_eq!(left_out, 2);
        assert_eq!(read_chat(&db, chat).unwrap().messages.len(), 6);
        assert!(matches!(prepared(&db, chat, &"x".repeat(1000)), Err(ChatError::Ai(AiError::TooLong { .. }))));
    }

    #[test]
    fn a_chat_without_an_agent_asks_nothing() {
        let (_directory, db) = database();
        let chat =
            create_chat(&db, &Chat { id: 0, title: "x".to_owned(), agent: None, work_unit: None, messages: vec![] })
                .unwrap();

        assert!(matches!(chat_question(&db, chat, "Hello"), Err(ChatError::Refused(_))));
    }

    #[test]
    fn an_answer_is_kept_with_the_mark_of_its_ending_and_a_failed_one_not_at_all() {
        let (_directory, db) = database();
        let local = create_integration(&db, &integration()).unwrap();
        let chat = new_chat(&db, "x", create_agent(&db, &agent(local)).unwrap()).unwrap();

        assert_eq!(add_answer(&db, chat, &answer("partial", Ending::Failed)).unwrap(), None);
        assert_eq!(add_answer(&db, chat, &answer("  ", Ending::Complete)).unwrap(), None);
        add_answer(&db, chat, &answer("half of it", Ending::Stopped)).unwrap();
        add_answer(&db, chat, &answer("", Ending::Cut)).unwrap();

        let texts: Vec<String> =
            read_chat(&db, chat).unwrap().messages.into_iter().map(|message| message.text).collect();

        assert_eq!(texts, ["half of it\n\n[stopped]", "[cut]"]);
    }
}
