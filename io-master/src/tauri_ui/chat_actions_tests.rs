//! Unit tests of `chat_actions.rs`, with a fake AI service.

use super::*;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::ai::api::api_key::ApiKey;
use crate::ai::api::test_support::{FakeAnswer, FakeService, completions, completions_events};
use crate::db::ai::connection::{create as connections_create, update as connections_update};
use crate::tauri_ui::test_support::{TestState, logged_in};
use crate::tauri_ui::user_data::with_user_db;

/// A logged-in state with an active connection to a fake service
/// that gives `answer`, and its API key.
async fn ready(answer: FakeAnswer) -> (TestState, FakeService) {
    let test = logged_in().await;
    let service = FakeService::start(answer).await;
    let connection = completions(&service);

    with_user_db(&test.state, move |db| {
        let id = connections_create::connection(db, &connection)?.id;
        connections_update::api_key(db, id, &ApiKey::new("sk-test".to_owned()).unwrap())?;
        Ok(())
    })
    .await
    .unwrap();

    (test, service)
}

/// Collects the events of a reply.
fn collector() -> (Arc<Mutex<Vec<ReplyEvent>>>, impl Fn(ReplyEvent) -> bool + Send) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&events);

    (events, move |event| {
        sink.lock().unwrap().push(event);
        true
    })
}

#[tokio::test]
async fn a_message_gets_a_streamed_reply_that_is_stored() {
    let (test, service) = ready(FakeAnswer::Stream(completions_events(&["ADV_FSP is ", "the functional spec"]))).await;
    let (events, sink) = collector();

    send_message(&test.state, None, "What is ADV_FSP?".into(), sink).await.unwrap();

    let events = events.lock().unwrap().clone();
    assert!(matches!(&events[0], ReplyEvent::Chat { id: 1, title } if title == "What is ADV_FSP?"));
    assert!(matches!(&events[1], ReplyEvent::Text { text } if text == "ADV_FSP is "));
    assert!(matches!(events.last(), Some(ReplyEvent::End { ending: "complete", notice: None })));

    let chat = open_chat(&test.state, 1).await.unwrap();
    assert_eq!(chat.messages.len(), 2);
    assert_eq!(chat.messages[1].text, "ADV_FSP is the functional spec");
    assert_eq!(chat.messages[1].who, "local-model");

    // The history goes with the next message.
    let (_, sink) = collector();
    send_message(&test.state, Some(1), "And ADV_TDS?".into(), sink).await.unwrap();
    // The instructions, the first message, the reply, the new one.
    assert_eq!(service.received()[1].body["messages"].as_array().unwrap().len(), 4);

    assert_eq!(list_chats(&test.state).await.unwrap().len(), 1);
}

#[tokio::test]
async fn stop_ends_the_reply_and_keeps_its_text() {
    let mut events = completions_events(&["First"]);
    events.truncate(1);

    let (test, _service) = ready(FakeAnswer::StreamThenWait(events)).await;
    let test = Arc::new(test);
    let (received, sink) = collector();

    let worker = Arc::clone(&test);
    let reply = tokio::spawn(async move { send_message(&worker.state, None, "Hello".into(), sink).await });

    // Wait for the first piece, then press "Stop".
    for _ in 0..100 {
        if received.lock().unwrap().iter().any(|event| matches!(event, ReplyEvent::Text { .. })) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    test.state.stop_reply();

    tokio::time::timeout(Duration::from_secs(10), reply).await.unwrap().unwrap().unwrap();

    let last = received.lock().unwrap().last().cloned();
    assert!(matches!(last, Some(ReplyEvent::End { ending: "stopped", .. })));

    let chat = open_chat(&test.state, 1).await.unwrap();
    assert_eq!(chat.messages[1].text, "First");
    assert_eq!(chat.messages[1].notice.as_deref(), Some("Stopped."));
}

#[tokio::test]
async fn a_refused_message_is_not_sent() {
    let (test, service) = ready(FakeAnswer::Stream(completions_events(&["x"]))).await;
    let (events, sink) = collector();

    let result = send_message(&test.state, Some(99), "Hello".into(), sink).await;

    assert!(matches!(result, Err(UiError::Invalid(_))));
    assert!(events.lock().unwrap().is_empty());
    assert!(service.received().is_empty());
}

#[tokio::test]
async fn a_chat_is_deleted() {
    let (test, _service) = ready(FakeAnswer::Stream(completions_events(&["Hi"]))).await;
    let (_, sink) = collector();

    send_message(&test.state, None, "Hello".into(), sink).await.unwrap();
    delete_chat(&test.state, 1).await.unwrap();

    assert!(list_chats(&test.state).await.unwrap().is_empty());
    assert!(matches!(delete_chat(&test.state, 1).await, Err(UiError::Invalid(_))));
}
