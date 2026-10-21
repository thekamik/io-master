//! Unit tests of `building.rs` (and of `proposals.rs`): the AI is a fake
//! service of the tests (`ai/api/test_support.rs`) that answers with a
//! fixed text.

use super::*;

use serde_json::json;

use crate::ai::api::connections;
use crate::ai::api::test_support::{FakeAnswer, FakeService, completions as connection_to, completions_events};
use crate::business::oscal::proposals;
use crate::db::encryption::database_key::DatabaseKey;
use crate::db::user::database as user_db;
use crate::model::api_key::ApiKey;
use crate::model::oscal::OscalModel;
use crate::security::random::fill_random;
use crate::test_support::TempDir;
use crate::test_support::standards::load;
use tokio::sync::watch;

/// A session of an evaluator whose AI is `service`, with the small
/// standard loaded and an empty catalog made from it; returns the
/// session, the catalog and the document.
fn ready(service: &FakeService, directory: &TempDir) -> (Arc<Session>, String, i64) {
    let key = DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap();
    let again = DatabaseKey::fill_with(|bytes| {
        bytes.copy_from_slice(key.expose());
        Ok::<(), ()>(())
    })
    .unwrap();
    let db = user_db::create(directory.path(), 1, &key).unwrap();

    let id = connections::create(&db, &connection_to(service)).unwrap().id;
    connections::set_key(&db, id, &ApiKey::new("sk-test".to_owned()).unwrap()).unwrap();
    let source = load(&db, b"%PDF building");
    let uuid = catalogs::create(&db, "CC Part 3", "", &[source]).unwrap().uuid;
    drop(db);

    let account = crate::installation::accounts::Account { id: 1, name: "anna".to_owned() };
    let session = Arc::new(Session::open(directory.path(), account, again).unwrap());

    (session, uuid, source)
}

/// A stop signal and its sender, which must live as long as the work:
/// a reply stops when its signal is closed (as when the window is gone).
fn cancel() -> (watch::Sender<bool>, Cancel) {
    let (sender, receiver) = watch::channel(false);

    (sender, Cancel::new(receiver))
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
    let directory = TempDir::new();
    let (session, uuid, source) = ready(&service, &directory);
    let events = Arc::new(std::sync::Mutex::new(Vec::new()));
    let seen = Arc::clone(&events);

    let (_keep, stop) = cancel();

    propose_groups(&session, uuid.clone(), source, 1, 3, stop, move |event| {
        seen.lock().unwrap().push(event);
        true
    })
    .await
    .unwrap();

    let events = events.lock().unwrap().clone();
    assert!(events.contains(&Progress::Proposed { added: 2, dropped: Vec::new() }), "{events:?}");
    assert_eq!(events.last(), Some(&Progress::End { stopped: false }));

    let db = session.db();
    let waiting = proposals::list(&db, &uuid).unwrap();
    assert_eq!(waiting.len(), 2);
    assert!(waiting[0].quotes[0].found);
    assert!(!waiting[1].quotes[0].found, "a made up heading is shown as such");
    // Nothing enters the catalog before the evaluator accepts it.
    assert!(catalogs::get(&db, &uuid).unwrap().groups.is_empty());

    assert_eq!(proposals::accept(&db, waiting[0].id, None).unwrap(), "ADV");
    assert!(proposals::reject(&db, waiting[1].id).unwrap());

    let catalog = catalogs::get(&db, &uuid).unwrap();
    assert_eq!(catalog.groups.len(), 1);
    assert_eq!(quotes::list(&db, &uuid, Some("ADV")).unwrap().len(), 1);
    assert!(proposals::list(&db, &uuid).unwrap().is_empty());

    // The question is in the journal of the AI.
    assert_eq!(crate::db::ai::journal::read::last(&db, 5).unwrap()[0].task, "catalog_outline");
}

#[tokio::test]
async fn the_controls_of_a_group_are_proposed_corrected_and_accepted() {
    let answer = json!({"controls": [{
        "identifier": "ADV_FSP.4", "title": "Complete functional specification",
        "statement": [{"identifier": "ADV_FSP.4.1D", "text": "The developer shall provide a functional specification."}],
        "objectives": [{"identifier": "ADV_FSP.4.1E", "text": "The evaluator shall confirm the information.", "method": "EXAMINE"}],
        "quotes": [{"page": 2, "text": "The developer shall provide a functional specification."}],
        "note": "From page 2."
    }]})
    .to_string();
    let service = FakeService::start(FakeAnswer::Stream(completions_events(&[&answer]))).await;
    let directory = TempDir::new();
    let (session, uuid, source) = ready(&service, &directory);

    {
        let db = session.db();
        catalogs::add_group(
            &db,
            &uuid,
            &catalogs::Place::Top,
            serde_json::from_value(json!({"id": "ADV", "title": "Development"})).unwrap(),
        )
        .unwrap();
    }

    let (_keep, stop) = cancel();

    propose_controls(&session, uuid.clone(), Some("ADV".to_owned()), vec![(source, 2, 3)], 0, stop, |_| true)
        .await
        .unwrap();

    let db = session.db();
    let waiting = proposals::list(&db, &uuid).unwrap();
    assert_eq!(waiting.len(), 1);
    assert_eq!(waiting[0].note, "From page 2.");
    assert!(waiting[0].quotes[0].found);

    // The evaluator corrects the title before accepting.
    let mut corrected = waiting[0].content.clone();
    corrected["title"] = json!("Complete functional specification (corrected)");
    proposals::accept(&db, waiting[0].id, Some(corrected)).unwrap();

    let catalog = catalogs::get(&db, &uuid).unwrap();
    let all = catalog.all_controls();
    assert_eq!(all[0].0, ["Development"]);
    assert_eq!(all[0].1.title, "Complete functional specification (corrected)");
    assert_eq!(quotes::list(&db, &uuid, Some("ADV_FSP.4")).unwrap().len(), 1);
    assert_eq!(crate::db::oscal::read::summaries(&db).unwrap()[0].model, OscalModel::Catalog);
}

#[tokio::test]
async fn a_proposal_that_breaks_a_rule_stays_and_nothing_is_written() {
    let service = FakeService::start(FakeAnswer::Stream(completions_events(&["{}"]))).await;
    let directory = TempDir::new();
    let (session, uuid, _) = ready(&service, &directory);
    let db = session.db();

    let content = json!({"id": "ADV_FSP.4", "title": "A"});
    let id = db_evaluation_support::create::proposal(
        &db,
        &NewProposal {
            document_uuid: &uuid,
            kind: ProposalKind::Control,
            parent_id: Some("NOT_THERE"),
            content: &content,
            quotes: &[],
            note: "",
            model: "m",
        },
    )
    .unwrap();

    assert!(matches!(proposals::accept(&db, id, None), Err(OscalError::NoSuchObject(_))));
    assert_eq!(proposals::list(&db, &uuid).unwrap().len(), 1, "the proposal waits, corrected later");
    assert!(catalogs::get(&db, &uuid).unwrap().all_controls().is_empty());
}
