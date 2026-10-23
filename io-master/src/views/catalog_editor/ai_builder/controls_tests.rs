//! Unit tests of `controls.rs` (and of a corrected and a refused
//! proposal), with a fake AI service.

use super::*;

use serde_json::json;

use crate::ai::api::test_support::{FakeAnswer, FakeService, completions_events};
use crate::model::oscal::catalog::edit::Place;
use crate::views::catalog_editor::ai_builder::groups::tests::{collector, ready};
use crate::views::catalog_editor::proposals::accept::accept;

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
    let (test, uuid, source) = ready(&service).await;
    let session = test.state.session().unwrap();

    {
        let db = session.db();
        let mut catalog = db_oscal::read::catalog(&db, &uuid).unwrap();
        catalog
            .add_group(&Place::Top, serde_json::from_value(json!({"id": "ADV", "title": "Development"})).unwrap())
            .unwrap();
        db_oscal::update::catalog(&db, catalog).unwrap();
    }

    let (_, channel) = collector();
    let ranges = [PageRange { document_id: source, first_page: 2, last_page: 3 }];

    propose(&session, &uuid, Some("ADV"), &ranges, 0, &test.state.start_work(), &channel).await.unwrap();

    let db = session.db();
    let waiting = db_evaluation_support::read::proposals(&db, &uuid).unwrap();
    assert_eq!(waiting.len(), 1);
    assert_eq!(waiting[0].note, "From page 2.");
    assert!(waiting[0].quotes[0].found);

    // The evaluator corrects the title before accepting.
    let mut corrected = waiting[0].content.clone();
    corrected["title"] = json!("Complete functional specification (corrected)");
    accept(&db, waiting[0].id, Some(corrected)).unwrap();

    let catalog = db_oscal::read::catalog(&db, &uuid).unwrap();
    let all = catalog.all_controls().unwrap();
    assert_eq!(all[0].0, ["Development"]);
    assert_eq!(all[0].1.title, "Complete functional specification (corrected)");
    assert_eq!(db_evaluation_support::read::quotes(&db, &uuid, Some("ADV_FSP.4")).unwrap().len(), 1);
}

#[tokio::test]
async fn a_proposal_that_breaks_a_rule_stays_and_nothing_is_written() {
    let service = FakeService::start(FakeAnswer::Stream(completions_events(&["{}"]))).await;
    let (test, uuid, _) = ready(&service).await;
    let session = test.state.session().unwrap();
    let db = session.db();

    let content = json!({"id": "ADV_FSP.4", "title": "A"});
    let proposal = NewProposal {
        document_uuid: &uuid,
        kind: ProposalKind::Control,
        parent_id: Some("NOT_THERE"),
        content: &content,
        quotes: &[],
        note: "",
        model: "m",
    };
    let id = db_evaluation_support::create::proposal(&db, &proposal).unwrap();

    assert!(matches!(accept(&db, id, None), Err(OscalError::NoSuchObject(_))));
    assert_eq!(db_evaluation_support::read::proposals(&db, &uuid).unwrap().len(), 1, "the proposal waits");
    assert!(db_oscal::read::catalog(&db, &uuid).unwrap().all_controls().unwrap().is_empty());
}

#[tokio::test]
async fn a_group_that_does_not_exist_and_no_pages_are_refused() {
    let service = FakeService::start(FakeAnswer::Stream(completions_events(&["{}"]))).await;
    let (test, uuid, source) = ready(&service).await;
    let session = test.state.session().unwrap();
    let (_, channel) = collector();
    let ranges = [PageRange { document_id: source, first_page: 1, last_page: 1 }];

    let missing = propose(&session, &uuid, Some("NOT_THERE"), &ranges, 0, &test.state.start_work(), &channel).await;
    let none = propose(&session, &uuid, None, &[], 0, &test.state.start_work(), &channel).await;

    assert!(matches!(missing, Err(AppError::Invalid(_))));
    assert!(matches!(none, Err(AppError::Invalid(_))));
}
