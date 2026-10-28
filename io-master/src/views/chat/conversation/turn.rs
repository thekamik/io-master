//! One turn of a chat: the message of the evaluator is checked and
//! stored, the model answers piece by piece (each piece goes to the page
//! as it arrives), and the reply is stored, also when it was stopped or
//! failed:
//!
//! ```text
//! prepare(db, chat, text)       check, store the message, read the history
//! knowledge(db, chat, agent)    the agent with knowledge of the chat kept; its
//!                               passages after the question (step 57.6), sent
//!                               only, never stored in the chat
//! completions::ask(...)         the reply, piece by piece, and the question
//!                               in the journal of the AI (ai/api/)
//! stored::add_reply(db, ...)    store it, also when stopped or failed
//! ```
//!
//! The chat is asked in the role *consultation* (`ai/api/roles.rs`), on the
//! active connection. The database is
//! held for each step only, never while the model writes.

use std::fmt;
use std::sync::Arc;

use tauri::ipc::Channel;
use tokio::sync::watch;

use crate::ai::api::completions;
use crate::ai::api::roles::{self, Model, ModelError};
use crate::ai::tasks::chat as memory;
use crate::ai::tasks::knowledge_context;
use crate::app::error::AppError;
use crate::components::knowledge_agent::find;
use crate::db_old::ai::chats as db_chats;
use crate::db_old::error::DbError;
use crate::db_old::knowledge as db_knowledge;
use crate::db_old::user::database::UserDb;
use crate::installation::session::Session;
use crate::model_old::ai::{ConnectionError, Task, Turn};
use crate::model_old::knowledge::AgentPlace;

use super::stored;
use super::view::{ReplyEvent, ending_notice};

/// The longest message an evaluator can send, in characters. A longer
/// text is better given to the model as a document (a later step).
pub const MAX_MESSAGE: usize = 100_000;

/// What a reply needs, made ready by [`prepare`].
#[derive(Debug)]
pub struct Prepared {
    /// The chat the message went to: the one asked for, or a new one.
    pub chat_id: i64,
    /// The title of that chat.
    pub title: String,
    /// The connection of the role *consultation*, checked, with its key.
    pub model: Model,
    /// The whole conversation as it is sent to the model, the new
    /// message last.
    pub turns: Vec<Turn>,
}

/// Why a message was not sent.
#[derive(Debug)]
pub enum SendError {
    /// The message is empty (or only spaces).
    Empty,
    /// The message is longer than [`MAX_MESSAGE`].
    TooLong,
    /// No connection is active.
    NoConnection,
    /// The active connection breaks a rule (a database changed by hand,
    /// or a rule of a newer version); holds its name and why.
    BrokenConnection(String, ConnectionError),
    /// The active connection uses an API key and none is stored; holds
    /// its name.
    NoKey(String),
    /// The chat asked for does not exist.
    NoSuchChat,
    /// The page can no longer be reached: nothing more is asked.
    PageGone,
    /// The database failed.
    Db(DbError),
    /// The knowledge of the agent could not be read; holds why.
    Knowledge(String),
}

impl fmt::Display for SendError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SendError::Empty => formatter.write_str("the message is empty"),
            SendError::TooLong => write!(formatter, "the message is longer than {MAX_MESSAGE} characters"),
            SendError::NoConnection => {
                formatter.write_str("no AI connection is chosen; set one up in Settings, tab AI")
            }
            SendError::BrokenConnection(name, error) => {
                write!(formatter, "the connection {name} cannot be used: {error}; correct it in Settings, tab AI")
            }
            SendError::NoKey(name) => write!(
                formatter,
                "the connection {name} uses an API key and none is stored; enter one in Settings, tab AI"
            ),
            SendError::NoSuchChat => formatter.write_str("the chat does not exist"),
            SendError::PageGone => formatter.write_str("the window can no longer be reached"),
            SendError::Db(error) => write!(formatter, "the message was not stored: {error}"),
            SendError::Knowledge(reason) => write!(formatter, "the knowledge of the agent could not be read: {reason}"),
        }
    }
}

impl std::error::Error for SendError {}

impl From<DbError> for SendError {
    fn from(error: DbError) -> SendError {
        SendError::Db(error)
    }
}

impl From<ModelError> for SendError {
    fn from(error: ModelError) -> SendError {
        match error {
            ModelError::NoConnection => SendError::NoConnection,
            ModelError::BrokenConnection(name, error) => SendError::BrokenConnection(name, error),
            ModelError::NoKey(name) => SendError::NoKey(name),
            ModelError::Db(error) => SendError::Db(error),
        }
    }
}

impl From<SendError> for AppError {
    fn from(error: SendError) -> AppError {
        match error {
            SendError::Db(_) | SendError::PageGone => AppError::Failed(error.to_string()),
            other => AppError::Invalid(other.to_string()),
        }
    }
}

/// Checks the message, stores it (in a new chat when `chat_id` is
/// `None`) and reads what the reply needs. Nothing is stored when the
/// message is refused.
pub fn prepare(db: &UserDb, chat_id: Option<i64>, text: &str) -> Result<Prepared, SendError> {
    let text = text.trim();

    ensure!(!text.is_empty(), SendError::Empty);
    ensure!(text.chars().count() <= MAX_MESSAGE, SendError::TooLong);

    // The connection first: without a usable one nothing is stored.
    let model = roles::model(db)?;

    let chat_id = match chat_id {
        Some(id) => {
            ensure!(db_chats::read::chat_title(db, id)?.is_some(), SendError::NoSuchChat);
            db_chats::create::message(db, id, text)?;
            id
        }
        None => stored::start(db, text)?,
    };

    Ok(Prepared {
        chat_id,
        title: db_chats::read::chat_title(db, chat_id)?.unwrap_or_default(),
        model,
        turns: memory::turns(&db_chats::read::messages(db, chat_id)?),
    })
}

/// Keeps `agent` as the agent with knowledge of chat `chat_id` (none:
/// `None`; an agent deleted since the page read it counts as none, the
/// message being stored already) and gives the context it adds to the
/// question `text`.
///
/// Called by: [`send`].
/// Calls: `db_knowledge::read::agent`, `db_knowledge::update::attachment`, `find::context`.
fn knowledge(db: &UserDb, chat_id: i64, agent: Option<i64>, text: &str) -> Result<String, SendError> {
    let agent = match agent {
        Some(id) if db_knowledge::read::agent(db, id)?.is_some() => Some(id),
        _ => None,
    };

    db_knowledge::update::attachment(db, AgentPlace::Chat, &chat_id.to_string(), agent)?;

    find::context(db, agent, text).map_err(|error| SendError::Knowledge(error.to_string()))
}

/// Sends `text` to chat `chat_id` (a new chat when `None`) and waits for
/// the reply, sending its events to `events` while it is written: the
/// chat first, then the pieces, then how it ended. A page that can no
/// longer be reached stops the reply as "Stop" (`stop`) does.
pub async fn send(
    session: &Session,
    chat_id: Option<i64>,
    text: &str,
    agent: Option<i64>,
    stop: &watch::Receiver<bool>,
    events: &Channel<ReplyEvent>,
) -> Result<(), SendError> {
    let Prepared { chat_id, title, model, mut turns } = prepare(&session.db(), chat_id, text)?;
    let context = knowledge(&session.db(), chat_id, agent, text)?;

    knowledge_context::into_last(&mut turns, &context);

    ensure!(events.send(ReplyEvent::Chat { id: chat_id, title }).is_ok(), SendError::PageGone);

    let model = Arc::new(model);
    let on_piece = |piece: &str| events.send(ReplyEvent::Text { text: piece.to_owned() }).is_ok();
    let asked = completions::ask(&session.db_handle(), &model, Task::Chat, &turns, stop, on_piece).await;

    let mut problems = Vec::new();

    if let Err(error) = stored::add_reply(&session.db(), chat_id, &asked.reply) {
        problems.push(format!("The reply was not stored: {error}."));
    }

    if let Err(error) = asked.journal {
        problems.push(format!("The question was not written into the journal of the AI: {error}."));
    }

    let (ending, note) = stored::ending_of(&asked.reply);
    let notice = with_problems(ending_notice(&ending, note.as_deref()), &problems);

    ensure!(events.send(ReplyEvent::End { ending: ending.name(), notice }).is_ok(), SendError::PageGone);

    Ok(())
}

/// The notice under a reply, with what could not be done afterwards.
fn with_problems(notice: Option<String>, problems: &[String]) -> Option<String> {
    if problems.is_empty() {
        return notice;
    }

    let problems = problems.join(" ");

    match notice {
        Some(before) => Some(format!("{before} {problems}")),
        None => Some(problems),
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests of `turn.rs`: [`prepare`] on a database, and [`send`] with a
    //! fake AI service and a channel that collects the events.

    use super::*;

    use std::sync::Mutex;
    use std::time::Duration;

    use serde_json::Value;
    use tauri::ipc::InvokeResponseBody;

    use crate::ai::api::test_support::{FakeAnswer, FakeService, completions, completions_events};
    use crate::app::test_support::{TestState, logged_in};

    use crate::db_old::ai::connection::{create as connections_create, update as connections_update};
    use crate::db_old::encryption::database_key::DatabaseKey;
    use crate::db_old::user::database as user_db;
    use crate::model_old::ai::Connection;
    use crate::model_old::ai::Speaker;
    use crate::model_old::api_key::ApiKey;
    use crate::security::random::fill_random;
    use crate::test_support::TempDir;

    /// A connection whose header uses the API key.
    fn with_key_header() -> Connection {
        Connection {
            name: "Local".to_owned(),
            model: "llama".to_owned(),
            headers: "authorization: Bearer {{api_key}}".to_owned(),
            ..Connection::template()
        }
    }

    /// A database of an evaluator with an active connection that uses an
    /// API key, and the key when `with_key`.
    fn database(directory: &TempDir, with_key: bool) -> UserDb {
        let key = DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap();
        let db = user_db::create(directory.path(), 1, &key).unwrap();
        let id = connections_create::connection(&db, &with_key_header()).unwrap().id;

        if with_key {
            connections_update::api_key(&db, id, &ApiKey::new("sk-test".to_owned()).unwrap()).unwrap();
        }

        db
    }

    #[test]
    fn a_first_message_starts_a_chat() {
        let directory = TempDir::new();
        let db = database(&directory, true);

        let prepared = prepare(&db, None, "  What is ADV_FSP?  ").unwrap();

        assert_eq!(prepared.title, "What is ADV_FSP?");
        assert_eq!(prepared.model.connection.model, "llama");
        assert_eq!(prepared.model.api_key.unwrap().expose(), "sk-test");
        assert_eq!(prepared.turns, [Turn { speaker: Speaker::Evaluator, text: "What is ADV_FSP?".to_owned() }]);
        assert_eq!(db_chats::read::chats(&db).unwrap().len(), 1);
    }

    #[test]
    fn a_further_message_continues_the_chat() {
        let directory = TempDir::new();
        let db = database(&directory, true);

        let first = prepare(&db, None, "first").unwrap();
        let second = prepare(&db, Some(first.chat_id), "second").unwrap();

        assert_eq!(second.chat_id, first.chat_id);
        assert_eq!(second.turns.len(), 2);
        assert_eq!(db_chats::read::chats(&db).unwrap().len(), 1);
    }

    #[test]
    fn a_connection_without_a_key_header_needs_no_key() {
        let directory = TempDir::new();
        let key = DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap();
        let db = user_db::create(directory.path(), 1, &key).unwrap();

        let without = Connection { headers: String::new(), ..with_key_header() };
        connections_create::connection(&db, &without).unwrap();

        assert!(prepare(&db, None, "Hello").unwrap().model.api_key.is_none());
    }

    #[test]
    fn refused_messages_store_nothing() {
        let directory = TempDir::new();
        let db = database(&directory, false);
        assert!(matches!(prepare(&db, None, "Hello"), Err(SendError::NoKey(name)) if name == "Local"));

        let empty = TempDir::new();
        let key = DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap();
        let without_connection = user_db::create(empty.path(), 1, &key).unwrap();
        assert!(matches!(prepare(&without_connection, None, "Hello"), Err(SendError::NoConnection)));

        let other = TempDir::new();
        let db = database(&other, true);
        assert!(matches!(prepare(&db, None, "   "), Err(SendError::Empty)));
        assert!(matches!(prepare(&db, None, &"x".repeat(MAX_MESSAGE + 1)), Err(SendError::TooLong)));
        assert!(matches!(prepare(&db, Some(99), "Hello"), Err(SendError::NoSuchChat)));
        assert!(db_chats::read::chats(&db).unwrap().is_empty());
    }

    #[test]
    fn a_connection_changed_by_hand_is_not_used() {
        let directory = TempDir::new();
        let db = database(&directory, true);

        db.connection_for_tests().execute("UPDATE ai_connections SET url = 'http://192.168.1.10/x'", []).unwrap();

        assert!(matches!(prepare(&db, None, "Hello"), Err(SendError::BrokenConnection(..))));
        assert!(db_chats::read::chats(&db).unwrap().is_empty());
    }

    /// A logged-in state with an active connection to a fake service that
    /// gives `answer`, and its API key.
    async fn ready(answer: FakeAnswer) -> (TestState, FakeService) {
        let test = logged_in().await;
        let service = FakeService::start(answer).await;

        {
            let session = test.state.session().unwrap();
            let db = session.db();
            let id = crate::ai::api::connections::create(&db, &completions(&service)).unwrap().id;
            connections_update::api_key(&db, id, &ApiKey::new("sk-test".to_owned()).unwrap()).unwrap();
        }

        (test, service)
    }

    /// A channel that collects the events it gets, as JSON.
    fn collector() -> (Arc<Mutex<Vec<Value>>>, Channel<ReplyEvent>) {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&events);

        let channel = Channel::new(move |body: InvokeResponseBody| {
            sink.lock().unwrap().push(body.deserialize::<Value>().unwrap());
            Ok(())
        });

        (events, channel)
    }

    /// The messages of chat `chat_id`.
    fn messages(test: &TestState, chat_id: i64) -> Vec<crate::model_old::chats::ChatMessage> {
        db_chats::read::messages(&test.state.session().unwrap().db(), chat_id).unwrap()
    }

    #[tokio::test]
    async fn a_message_gets_a_streamed_reply_that_is_stored() {
        let (test, service) =
            ready(FakeAnswer::Stream(completions_events(&["ADV_FSP is ", "the functional spec"]))).await;
        let session = test.state.session().unwrap();
        let (events, channel) = collector();

        send(&session, None, "What is ADV_FSP?", None, &test.state.start_reply(), &channel).await.unwrap();

        let events = events.lock().unwrap().clone();
        assert_eq!(events[0], serde_json::json!({ "event": "chat", "id": 1, "title": "What is ADV_FSP?" }));
        assert_eq!(events[1], serde_json::json!({ "event": "text", "text": "ADV_FSP is " }));
        assert_eq!(
            events.last().unwrap(),
            &serde_json::json!({ "event": "end", "ending": "complete", "notice": null })
        );

        let stored = messages(&test, 1);
        assert_eq!(stored.len(), 2);
        assert_eq!(stored[1].text, "ADV_FSP is the functional spec");
        assert_eq!(stored[1].model.as_deref(), Some("local-model"));

        // The history goes with the next message: the instructions, the
        // first message, the reply, the new one.
        let (_, channel) = collector();
        send(&session, Some(1), "And ADV_TDS?", None, &test.state.start_reply(), &channel).await.unwrap();
        assert_eq!(service.received()[1].body["messages"].as_array().unwrap().len(), 4);
    }

    #[tokio::test]
    async fn stop_ends_the_reply_and_keeps_its_text() {
        let mut answer = completions_events(&["First"]);
        answer.truncate(1);

        let (test, _service) = ready(FakeAnswer::StreamThenWait(answer)).await;
        let test = Arc::new(test);
        let (received, channel) = collector();

        let worker = Arc::clone(&test);
        let reply = tokio::spawn(async move {
            let session = worker.state.session().unwrap();
            send(&session, None, "Hello", None, &worker.state.start_reply(), &channel).await
        });

        // Wait for the first piece, then press "Stop".
        for _ in 0..100 {
            if received.lock().unwrap().iter().any(|event| event["event"] == "text") {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        test.state.stop_reply();

        tokio::time::timeout(Duration::from_secs(10), reply).await.unwrap().unwrap().unwrap();

        assert_eq!(received.lock().unwrap().last().unwrap()["ending"], "stopped");

        let stored = messages(&test, 1);
        assert_eq!(stored[1].text, "First");
        assert_eq!(stored[1].ending, crate::model_old::chats::Ending::Stopped);
    }

    #[tokio::test]
    async fn a_refused_message_is_not_sent() {
        let (test, service) = ready(FakeAnswer::Stream(completions_events(&["x"]))).await;
        let session = test.state.session().unwrap();
        let (events, channel) = collector();

        let result = send(&session, Some(99), "Hello", None, &test.state.start_reply(), &channel).await;

        assert!(matches!(result, Err(SendError::NoSuchChat)));
        assert!(events.lock().unwrap().is_empty());
        assert!(service.received().is_empty());
    }

    #[tokio::test]
    async fn the_agent_of_the_chat_is_kept_and_its_passages_follow_the_question() {
        let (test, service) = ready(FakeAnswer::Stream(completions_events(&["Roles are allocated."]))).await;
        let session = test.state.session().unwrap();
        let text = "Information security roles and responsibilities are allocated by the management.";
        let agent = crate::test_support::db::knowledge_agent(&session.db(), text);
        let (_, channel) = collector();

        send(&session, None, "Who allocates the security roles?", Some(agent), &test.state.start_reply(), &channel)
            .await
            .unwrap();

        let asked = service.received()[0].body.to_string();
        assert!(asked.contains("The context of the standard") && asked.contains(text));
        let db = session.db();
        assert_eq!(db_knowledge::read::attachment(&db, AgentPlace::Chat, "1").unwrap(), Some(agent));
        assert_eq!(db_chats::read::messages(&db, 1).unwrap()[0].text, "Who allocates the security roles?");
    }

    #[tokio::test]
    async fn an_agent_deleted_since_the_page_read_it_counts_as_none() {
        let (test, service) = ready(FakeAnswer::Stream(completions_events(&["Yes."]))).await;
        let session = test.state.session().unwrap();
        let (_, channel) = collector();

        send(&session, None, "Hello", Some(99), &test.state.start_reply(), &channel).await.unwrap();

        assert!(!service.received()[0].body.to_string().contains("The context of the standard"));
        assert_eq!(db_knowledge::read::attachment(&session.db(), AgentPlace::Chat, "1").unwrap(), None);
    }
}
