//! The chat (user, 2026-10-30) as the AI works on it: the question of a
//! chat, with the instructions and the knowledge of its agent and as many of
//! the latest messages of the chat as fit into the context of the model; the
//! answer written into the chat. The rules of a chat (made, renamed, a
//! question written) are in `domain/models/ai/chat.rs`.
//!
//! ```text
//! chat_question ──► the instructions of the agent
//!                   + the knowledge of the agent that fits the question (RAG)
//!                   + every message of the chat and the question ──► Question (integration/question.rs)
//! prepared      ──► prepare_shortened: prepare; the oldest messages left out until the question fits
//! add_answer    ──► the answer written into the chat; a stopped or cut one with a mark,
//!                   a failed one not at all (its reason is in the journal of the agent)
//! ```
//!
//! **The knowledge** is looked for with the question and the question of the
//! user before it, so that a short question that goes on with the earlier
//! one ("and in the ST?") still finds what the conversation is about.
//!
//! **A long chat is shortened** (a chat is no part of an evaluation): when
//! the question does not fit, its oldest messages are left out of it, never
//! the instructions, the knowledge or the question; the page is told how
//! many. The chat keeps them all, and the journal of the agent keeps the
//! question as it was sent.

use crate::ai::error::AiError;
use crate::ai::models::ai::agent::agent_question;
use crate::ai::models::ai::integration::question::{MAX_MESSAGES, Prepared, Question, prepare};
use crate::ai::models::ai::journal::Answer;
use crate::db::models::ai::chat::{MAX_MESSAGES as MAX_CHAT_MESSAGES, create_message, read_chat};
use crate::domain::models::ai::chat::checked_question;
use crate::infrastructure::clock::now;
use crate::infrastructure::storage::user_database::UserDatabase;
use crate::model::ai::chat::{Message, Speaker};
use crate::model::ai::journal::Ending;

/// The marks written after an answer that did not end by itself.
const STOPPED: &str = "[stopped]";
const CUT: &str = "[cut]";
const REFUSED: &str = "[refused]";

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
/// Called by: [`prepared`].
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
/// Calls: `db/models/ai/chat.rs`: `read_chat`; `domain/models/ai/chat.rs`:
/// `checked_question`; [`search_text`]; `agent.rs`: `agent_question`.
fn chat_question(db: &UserDatabase, chat: i64, text: &str) -> Result<Question, AiError> {
    let stored = read_chat(db, chat)?;
    let question = checked_question(text)?;
    let Some(agent_id) = stored.agent else {
        return Err(AiError::Refused("the chat has no agent".to_owned()));
    };

    ensure!(
        stored.messages.len() + 2 <= MAX_CHAT_MESSAGES,
        AiError::Refused(format!("the chat is full ({MAX_CHAT_MESSAGES} messages): begin a new one"))
    );

    let search = search_text(&stored.messages, &question);
    let mut messages = stored.messages;

    messages.push(Message { speaker: Speaker::User, text: question, time: 0 });

    agent_question(db, agent_id, "", &search, None, messages)
}

/// The question `text` in the chat `chat`, prepared for the integration of
/// its agent, with as many of the latest messages of the chat as fit into
/// the context of the model; gives also how many messages were left out.
/// Refused when the question does not fit even alone.
///
/// Called by: `panels/views/chat/chats.rs`: `chat_ask`.
/// Calls: [`chat_question`], [`prepare_shortened`].
pub fn prepared(db: &UserDatabase, chat: i64, text: &str) -> Result<(Prepared, usize), AiError> {
    prepare_shortened(db, chat_question(db, chat, text)?)
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
/// Called by: `panels/views/chat/chats.rs`: `chat_ask` (after `journal.rs`:
/// `record`).
/// Calls: [`kept_text`]; `infrastructure/clock.rs`: `now`; `db/models/ai/chat.rs`:
/// `create_message`.
pub fn add_answer(db: &UserDatabase, chat: i64, answer: &Answer) -> Result<Option<Message>, AiError> {
    let Some(text) = kept_text(answer) else {
        return Ok(None);
    };
    let message =
        Message { speaker: Speaker::Ai, text, time: now().map_err(|error| AiError::Refused(error.to_string()))? };

    create_message(db, chat, &message)?;

    Ok(Some(message))
}

#[cfg(test)]
mod tests {
    //! Tests of a question of a chat: shortened until it fits, with the
    //! instructions, the knowledge and the conversation; the answers kept
    //! with their marks.

    use super::*;

    use crate::db::models::ai::agent::create_agent;
    use crate::db::models::ai::chat::create_chat;
    use crate::db::models::ai::integration::create_integration;
    use crate::db::models::ai::rag::{create_knowledge_base, create_source};
    use crate::domain::error::DomainError;
    use crate::domain::models::ai::chat::{add_question, new_chat};
    use crate::model::ai::chat::Chat;
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
        assert!(matches!(chat_question(&db, chat, " "), Err(AiError::Domain(DomainError::Refused(_)))));
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
        assert!(matches!(prepared(&db, chat, &"x".repeat(1000)), Err(AiError::TooLong { .. })));
    }

    #[test]
    fn a_chat_without_an_agent_asks_nothing() {
        let (_directory, db) = database();
        let chat =
            create_chat(&db, &Chat { id: 0, title: "x".to_owned(), agent: None, work_unit: None, messages: vec![] })
                .unwrap();

        assert!(matches!(chat_question(&db, chat, "Hello"), Err(AiError::Refused(_))));
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
