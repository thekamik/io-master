//! Unit tests of `completions.rs`, against the fake service of
//! `test_support.rs` (no network).

use super::*;

use tokio::sync::watch;

use crate::ai::api::test_support::{
    FakeAnswer, FakeService, checked, completions as completions_format, completions_events, data,
};
use crate::db::ai::journal::read as journal_read;
use crate::model::api_key::ApiKey;
use crate::test_projects::test_support::database;

/// The first piece of a reply that never ends: the service then stays
/// silent until the client gives up.
fn unfinished() -> FakeAnswer {
    FakeAnswer::StreamThenWait(vec![data(&serde_json::json!({"choices": [{"delta": {"content": "Hel"}}]}).to_string())])
}

/// The model of the fake service, with a key.
fn model(service: &FakeService) -> Model {
    Model {
        name: "Local".to_owned(),
        connection: checked(completions_format(service)),
        api_key: Some(ApiKey::new("sk-test-secret".to_owned()).unwrap()),
    }
}

#[tokio::test]
async fn a_reply_arrives_piece_by_piece() {
    let service = FakeService::start(FakeAnswer::Stream(completions_events(&["Hel", "lo"]))).await;
    let (_stop, receiver) = watch::channel(false);
    let mut pieces = Vec::new();

    let reply = create(&model(&service), &single("Hi".to_owned()), &receiver, |piece| {
        pieces.push(piece.to_owned());
        true
    })
    .await;

    assert_eq!(pieces, ["Hel", "lo"]);
    assert_eq!(reply.text, "Hello");
    assert_eq!(outcome(&reply, &receiver), Outcome::Answer("Hello".to_owned()));
    assert_eq!(ending(&reply), ("complete", None));
}

#[tokio::test]
async fn stop_ends_a_reply() {
    let service = FakeService::start(unfinished()).await;
    let (stop, receiver) = watch::channel(false);

    let reply = create(&model(&service), &single("Hi".to_owned()), &receiver, |_| {
        stop.send(true).unwrap();
        true
    })
    .await;

    assert_eq!(reply.end, ReplyEnd::Cancelled);
    assert_eq!(outcome(&reply, &receiver), Outcome::Stopped);
    assert_eq!(ending(&reply).0, "stopped");
}

#[tokio::test]
async fn a_caller_that_wants_no_more_stops_the_reply() {
    let service = FakeService::start(unfinished()).await;
    let (_stop, receiver) = watch::channel(false);

    let reply = create(&model(&service), &single("Hi".to_owned()), &receiver, |_| false).await;

    assert_eq!(reply.end, ReplyEnd::Cancelled);
}

#[test]
fn counting_reports_the_characters_written() {
    let reports = std::cell::RefCell::new(Vec::new());
    let mut on_piece = counting(5, |characters| reports.borrow_mut().push(characters));

    for piece in ["abc", "de", "f", "ghijk"] {
        assert!(on_piece(piece));
    }

    assert_eq!(*reports.borrow(), [5, 11]);
}

#[tokio::test]
async fn a_question_is_recorded_without_the_key() {
    let service = FakeService::start(FakeAnswer::Stream(completions_events(&["OK"]))).await;
    let (_stop, receiver) = watch::channel(false);
    let model = model(&service);
    let turns = single("Read the table of contents.".to_owned());

    let reply = create(&model, &turns, &receiver, |_| true).await;

    let test = database();
    record(&test.db, &model, Task::Contents, &turns, &reply).unwrap();

    let entry = &journal_read::last(&test.db, 1).unwrap()[0];
    assert_eq!((entry.task.as_str(), entry.role.as_str()), ("contents", "text_analysis"));
    assert_eq!((entry.connection.as_str(), entry.model.as_str()), ("Local", "local-model"));
    assert!(entry.request.contains("Read the table of contents."), "{}", entry.request);
    assert!(!entry.request.contains("sk-test-secret"));
    assert_eq!((entry.answer.as_str(), entry.ending.as_str()), ("OK", "complete"));
    assert_eq!((entry.input_tokens, entry.output_tokens), (10, 5));
}
