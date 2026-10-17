//! Unit tests of `project_actions.rs`, with small PDFs, the real tools
//! of Poppler and a fake AI service.

use super::*;

use std::sync::{Arc, Mutex};

use crate::ai::api::api_key::ApiKey;
use crate::ai::api::test_support::{FakeAnswer, FakeService, completions, completions_events};
use crate::tauri_ui::scheme_actions;
use crate::tauri_ui::serializer::scheme_view::TileForm;
use crate::tauri_ui::test_support::{logged_in, project_form, scheme_form, with_document};
use crate::tpm::test_support::TempDir;

fn input(title: &str) -> TileForm {
    TileForm {
        kind: "input".into(),
        identifier: String::new(),
        title: title.into(),
        description: String::new(),
        place: String::new(),
        state: "accepted".into(),
        external: None,
        sections: Vec::new(),
        instructions: String::new(),
        x: 0.0,
        y: 0.0,
        sources: Vec::new(),
    }
}

/// The active AI connection of the state: a fake service that answers
/// `answer` in pieces.
async fn fake_model(state: &UiState, answer: &str) -> FakeService {
    let pieces: Vec<String> =
        answer.as_bytes().chunks(40).map(|chunk| String::from_utf8_lossy(chunk).into_owned()).collect();
    let pieces: Vec<&str> = pieces.iter().map(String::as_str).collect();
    let service = FakeService::start(FakeAnswer::Stream(completions_events(&pieces))).await;

    let connection = completions(&service);
    with_user_db(state, move |db| {
        let id = crate::db::ai::connection::create::connection(db, &connection)?.id;
        crate::db::ai::connection::update::api_key(db, id, &ApiKey::new("sk-test".to_owned()).unwrap())?;
        Ok(())
    })
    .await
    .unwrap();

    service
}

#[tokio::test]
async fn a_project_holds_its_schemes_and_their_counts() {
    let (test, document) = with_document().await;
    let project = create(&test.state, project_form("ADV", vec![document])).await.unwrap().project.id;

    let view =
        create_scheme(&test.state, project, scheme_form("5.1", "Specification", Some((document, 1, 2)))).await.unwrap();
    let scheme = view.schemes[0].id;
    assert_eq!(view.schemes[0].pages.map(|pages| pages.last_page), Some(2));

    scheme_actions::add_tile(&test.state, scheme, input("Sample")).await.unwrap();

    let view = open(&test.state, project).await.unwrap();
    assert_eq!((view.schemes[0].tiles, view.schemes[0].findings), (1, 1));
    assert_eq!(view.findings, 1);
    assert!(view.can_undo);

    let view = update_scheme(&test.state, scheme, scheme_form("5.1", "Functional specification", None)).await.unwrap();
    assert_eq!(view.schemes[0].name, "Functional specification");

    let ready = set_ready(&test.state, project, true).await.unwrap();
    assert!(ready.project.ready);
    assert_eq!(list(&test.state).await.unwrap().len(), 1);

    // Deleting a scheme forgets "Undo": an earlier content names its tiles.
    let view = delete_schemes(&test.state, project, vec![scheme]).await.unwrap();
    assert!(view.schemes.is_empty());
    assert!(!view.can_undo);

    delete(&test.state, project).await.unwrap();
    assert!(list(&test.state).await.unwrap().is_empty());
}

#[tokio::test]
async fn a_project_goes_to_a_file_and_back() {
    let (test, document) = with_document().await;
    let project = create(&test.state, project_form("To share", vec![document])).await.unwrap().project.id;
    let scheme = create_scheme(&test.state, project, scheme_form("1", "S", None)).await.unwrap().schemes[0].id;
    scheme_actions::add_tile(&test.state, scheme, input("Sample")).await.unwrap();

    let directory = TempDir::new();
    let file = directory.path().join("project.json");
    export(&test.state, project, true, &file).await.unwrap();

    // Another evaluator, without the document: the PDF in the file
    // loads it.
    let other = logged_in().await;
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&events);
    let imported = import(&other.state, &file, move |event| {
        sink.lock().unwrap().push(event);
        true
    })
    .await
    .unwrap();

    assert_eq!(imported.project.name, "To share");
    assert_eq!(imported.schemes.len(), 1);
    assert_eq!(imported.schemes[0].tiles, 1);
    assert_eq!(imported.documents.len(), 1);
    assert!(events.lock().unwrap().iter().any(|event| matches!(event, LoadEvent::Pages { .. })));

    // Without the PDF, the missing document is named.
    let lean = directory.path().join("lean.json");
    export(&test.state, project, false, &lean).await.unwrap();
    let third = logged_in().await;

    let refused = import(&third.state, &lean, |_| true).await;
    assert!(matches!(refused, Err(UiError::Invalid(reason)) if reason.contains("cc-part3.pdf")));
}

#[tokio::test]
async fn the_table_of_contents_becomes_proposed_schemes() {
    let (test, document) = with_document().await;
    let project = create(&test.state, project_form("CC", vec![document])).await.unwrap().project.id;

    // The model names two chapters; their headings are on pages 1 and 3.
    let answer = r#"{"chapters": [
        {"number": "ADV_FSP.4", "title": "Complete functional specification", "page": 1},
        {"number": "ATE_IND.2", "title": "The evaluator shall test", "page": null}]}"#;
    let service = fake_model(&test.state, answer).await;

    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&events);
    let range = PageRange { document_id: document, first_page: 1, last_page: 1 };

    // The table of contents is page 1 here, so the headings are looked
    // for from page 2: the first chapter is not found, the second is.
    let view = propose_schemes(&test.state, project, range, move |event| {
        sink.lock().unwrap().push(event);
        true
    })
    .await
    .unwrap();

    assert_eq!(view.schemes.len(), 2);
    assert!(view.schemes.iter().all(|scheme| scheme.state == "proposed"));
    assert_eq!(view.schemes[1].pages.map(|pages| (pages.first_page, pages.last_page)), Some((3, 3)));

    let events = events.lock().unwrap().clone();
    assert!(events.iter().any(|event| matches!(event, AnalysisEvent::Chapters { added: 2, known: 0, .. })));
    assert!(matches!(events.last(), Some(AnalysisEvent::End { stopped: false })));

    // The instructions of the table of contents went to the model.
    let sent = service.received()[0].body["messages"][1]["content"].as_str().unwrap().to_owned();
    assert!(sent.starts_with("You help to map a standard"));

    let view = accept_schemes(&test.state, project, vec![view.schemes[0].id]).await.unwrap();
    assert_eq!(view.schemes[0].state, "accepted");
}
