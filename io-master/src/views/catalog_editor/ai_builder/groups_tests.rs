//! Unit tests of `groups.rs` (and of the acceptance of a proposal): the
//! AI is a fake service of the tests (`ai/api/test_support.rs`) that
//! answers with a fixed text; the events go into a channel that collects
//! them.

use super::*;

use std::sync::Mutex;

use serde_json::{Value, json};
use tauri::ipc::InvokeResponseBody;

use crate::ai::api::connections;
use crate::ai::api::test_support::{FakeAnswer, FakeService, completions as connection_to, completions_events};
use crate::app::test_support::{TestState, logged_in};
use crate::model::api_key::ApiKey;
use crate::test_support::standards::load;
use crate::views::catalog_editor::proposals::accept::accept;
use crate::views::plans::new_catalog::made;

/// "anna" logged in, her AI the fake `service`, the small standard loaded
/// and an empty catalog made from it; returns the state, the catalog and
/// the document.
pub async fn ready(service: &FakeService) -> (TestState, String, i64) {
    let test = logged_in().await;

    let (uuid, source) = {
        let session = test.state.session().unwrap();
        let db = session.db();
        let id = connections::create(&db, &connection_to(service)).unwrap().id;
        connections::set_key(&db, id, &ApiKey::new("sk-test".to_owned()).unwrap()).unwrap();
        let source = load(&db, b"%PDF building");

        (made::create(&db, "CC Part 3", "", &[source]).unwrap().uuid, source)
    };

    (test, uuid, source)
}

/// A channel that collects the events it gets, as JSON.
pub fn collector() -> (Arc<Mutex<Vec<Value>>>, Channel<BuildEvent>) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&events);

    let channel = Channel::new(move |body: InvokeResponseBody| {
        sink.lock().unwrap().push(body.deserialize::<Value>().unwrap());
        Ok(())
    });

    (events, channel)
}

#[tokio::test]
async fn the_groups_of_a_table_of_contents_are_proposed_and_accepted() {
    let answer = json!({"groups": [
        {"identifier": "ADV", "title": "Development", "parent": null,
         "quote": {"page": 2, "text": "ADV_FSP.4 Complete functional specification"}},
        {"identifier": "ATE", "title": "Tests", "parent": null, "quote": {"page": 3, "text": "Made up heading"}}
    ]})
    .to_string();
    let service = FakeService::start(FakeAnswer::Stream(completions_events(&[&answer]))).await;
    let (test, uuid, source) = ready(&service).await;
    let session = test.state.session().unwrap();
    let (events, channel) = collector();
    let range = PageRange { document_id: source, first_page: 1, last_page: 3 };

    propose(&session, &uuid, &range, &test.state.start_work(), &channel).await.unwrap();

    let events = events.lock().unwrap().clone();
    assert!(events.contains(&json!({ "event": "proposed", "added": 2, "dropped": [] })), "{events:?}");
    assert_eq!(events.last().unwrap(), &json!({ "event": "end", "stopped": false }));

    let db = session.db();
    let waiting = db_evaluation_support::read::proposals(&db, &uuid).unwrap();
    assert_eq!(waiting.len(), 2);
    assert!(waiting[0].quotes[0].found);
    assert!(!waiting[1].quotes[0].found, "a made up heading is shown as such");
    // Nothing enters the catalog before the evaluator accepts it.
    assert!(db_oscal::read::catalog(&db, &uuid).unwrap().groups.is_empty());

    assert_eq!(accept(&db, waiting[0].id, None).unwrap(), "ADV");
    assert!(db_evaluation_support::delete::proposal(&db, waiting[1].id).unwrap());

    assert_eq!(db_oscal::read::catalog(&db, &uuid).unwrap().groups.len(), 1);
    assert_eq!(db_evaluation_support::read::quotes(&db, &uuid, Some("ADV")).unwrap().len(), 1);
    assert!(db_evaluation_support::read::proposals(&db, &uuid).unwrap().is_empty());

    // The question is in the journal of the AI.
    assert_eq!(crate::db::ai::journal::read::last(&db, 5).unwrap()[0].task, "catalog_outline");
}

#[tokio::test]
async fn stop_ends_the_work_before_a_question() {
    let service = FakeService::start(FakeAnswer::Stream(completions_events(&["{}"]))).await;
    let (test, uuid, source) = ready(&service).await;
    let session = test.state.session().unwrap();
    let (events, channel) = collector();
    let range = PageRange { document_id: source, first_page: 1, last_page: 3 };

    let stop = test.state.start_work();
    test.state.stop_work();
    propose(&session, &uuid, &range, &stop, &channel).await.unwrap();

    assert_eq!(events.lock().unwrap().last().unwrap(), &json!({ "event": "end", "stopped": true }));
    assert!(service.received().is_empty());
}
