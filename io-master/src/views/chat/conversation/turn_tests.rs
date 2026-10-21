//! Unit tests of `turn.rs`: [`prepare`] on a database, and [`send`] with a
//! fake AI service and a channel that collects the events.

use super::*;

use std::sync::Mutex;
use std::time::Duration;

use serde_json::Value;
use tauri::ipc::InvokeResponseBody;

use crate::ai::api::test_support::{FakeAnswer, FakeService, completions, completions_events};
use crate::tauri_ui::test_support::{TestState, logged_in};

use crate::db::ai::connection::{create as connections_create, update as connections_update};
use crate::db::encryption::database_key::DatabaseKey;
use crate::db::user::database as user_db;
use crate::model::ai::Connection;
use crate::model::ai::Speaker;
use crate::model::api_key::ApiKey;
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
fn messages(test: &TestState, chat_id: i64) -> Vec<crate::model::chats::ChatMessage> {
    db_chats::read::messages(&test.state.session().unwrap().db(), chat_id).unwrap()
}

#[tokio::test]
async fn a_message_gets_a_streamed_reply_that_is_stored() {
    let (test, service) = ready(FakeAnswer::Stream(completions_events(&["ADV_FSP is ", "the functional spec"]))).await;
    let session = test.state.session().unwrap();
    let (events, channel) = collector();

    send(&session, None, "What is ADV_FSP?", &test.state.start_reply(), &channel).await.unwrap();

    let events = events.lock().unwrap().clone();
    assert_eq!(events[0], serde_json::json!({ "event": "chat", "id": 1, "title": "What is ADV_FSP?" }));
    assert_eq!(events[1], serde_json::json!({ "event": "text", "text": "ADV_FSP is " }));
    assert_eq!(events.last().unwrap(), &serde_json::json!({ "event": "end", "ending": "complete", "notice": null }));

    let stored = messages(&test, 1);
    assert_eq!(stored.len(), 2);
    assert_eq!(stored[1].text, "ADV_FSP is the functional spec");
    assert_eq!(stored[1].model.as_deref(), Some("local-model"));

    // The history goes with the next message: the instructions, the
    // first message, the reply, the new one.
    let (_, channel) = collector();
    send(&session, Some(1), "And ADV_TDS?", &test.state.start_reply(), &channel).await.unwrap();
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
        send(&session, None, "Hello", &worker.state.start_reply(), &channel).await
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
    assert_eq!(stored[1].ending, crate::model::chats::Ending::Stopped);
}

#[tokio::test]
async fn a_refused_message_is_not_sent() {
    let (test, service) = ready(FakeAnswer::Stream(completions_events(&["x"]))).await;
    let session = test.state.session().unwrap();
    let (events, channel) = collector();

    let result = send(&session, Some(99), "Hello", &test.state.start_reply(), &channel).await;

    assert!(matches!(result, Err(SendError::NoSuchChat)));
    assert!(events.lock().unwrap().is_empty());
    assert!(service.received().is_empty());
}
