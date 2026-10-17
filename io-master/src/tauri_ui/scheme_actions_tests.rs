//! Unit tests of `scheme_actions.rs`, with small PDFs, the real tools
//! of Poppler and a fake AI service.

use super::*;

use std::sync::{Arc, Mutex};

use crate::ai::api::test_support::{FakeAnswer, FakeService, completions, completions_events};
use crate::ai::tasks::chapter::DEFAULT_INSTRUCTIONS;
use crate::ai::tasks::contents;
use crate::model::api_key::ApiKey;
use crate::tauri_ui::project_actions;
use crate::tauri_ui::serializer::scheme_view::SourceView;
use crate::tauri_ui::test_support::{TestState, logged_in, project_form, scheme_form, with_document};

/// A project "P" on the document with the schemes `numbers` (each
/// covering the whole document); returns the project and the schemes.
async fn project_with(test: &TestState, document: i64, numbers: &[&str]) -> (i64, Vec<i64>) {
    let project = project_actions::create(&test.state, project_form("P", vec![document])).await.unwrap().project.id;
    let mut schemes = Vec::new();

    for number in numbers {
        let view = project_actions::create_scheme(
            &test.state,
            project,
            scheme_form(number, "Chapter", Some((document, 1, 3))),
        )
        .await
        .unwrap();
        schemes.push(view.schemes.last().unwrap().id);
    }

    (project, schemes)
}

fn tile(kind: &str, title: &str) -> TileForm {
    TileForm {
        kind: kind.into(),
        identifier: String::new(),
        title: title.into(),
        description: String::new(),
        place: String::new(),
        state: "accepted".into(),
        external: None,
        sections: if kind == "output" { vec!["requirements".into(), "verdict".into()] } else { Vec::new() },
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
async fn a_scheme_is_drawn_checked_and_undone() {
    let (test, document) = with_document().await;
    let (_, schemes) = project_with(&test, document, &["5"]).await;
    let scheme = schemes[0];

    let input = add_tile(&test.state, scheme, tile("input", "Functional specification")).await.unwrap();
    assert_eq!(input.findings.len(), 1);
    assert!(input.can_undo);
    let input = input.tiles[0].id;

    let mut requirement = tile("requirement", "ADV_FSP.4");
    requirement.sources = vec![SourceView {
        document_id: document,
        page: 2,
        quote: "The developer shall provide a functional specification.".into(),
        found: false,
    }];
    let view = add_tile(&test.state, scheme, requirement).await.unwrap();
    let requirement = view.tiles[1].id;
    assert!(view.tiles[1].fields.sources[0].found);

    let view = add_tile(&test.state, scheme, tile("output", "Report")).await.unwrap();
    let output = view.tiles[2].id;

    add_link(&test.state, scheme, input, requirement).await.unwrap();
    let linked = add_link(&test.state, scheme, requirement, output).await.unwrap();
    assert!(linked.findings.is_empty(), "{:?}", linked.findings);
    assert_eq!(linked.links[1].meaning, "reported_in");
    assert_eq!(linked.scheme.findings, 0);

    assert!(matches!(add_link(&test.state, scheme, input, output).await, Err(UiError::Invalid(_))));

    // Undo takes the last link back.
    let undone = undo(&test.state, scheme).await.unwrap();
    assert_eq!(undone.links.len(), 1);

    move_tile(&test.state, undone.tiles[0].id, 50.0, 60.0).await.unwrap();
    let moved = open(&test.state, scheme).await.unwrap();
    assert_eq!((moved.tiles[0].fields.x, moved.tiles[0].fields.y), (50.0, 60.0));
}

#[tokio::test]
async fn proposed_tiles_are_accepted_or_deleted() {
    let (test, document) = with_document().await;
    let (_, schemes) = project_with(&test, document, &["5"]).await;
    let scheme = schemes[0];

    let mut proposed = tile("input", "Sample");
    proposed.state = "proposed".into();
    add_tile(&test.state, scheme, proposed).await.unwrap();
    let mut other = tile("input", "Other");
    other.state = "proposed".into();
    let view = add_tile(&test.state, scheme, other).await.unwrap();
    let (first, second) = (view.tiles[0].id, view.tiles[1].id);

    let view = accept_tiles(&test.state, scheme, vec![first]).await.unwrap();
    assert_eq!(view.tiles[0].fields.state, "accepted");

    let view = delete_tiles(&test.state, scheme, vec![second]).await.unwrap();
    assert_eq!(view.tiles.len(), 1);
}

#[tokio::test]
async fn tiles_of_other_schemes_are_shadows_and_tiles_move_between_schemes() {
    let (test, document) = with_document().await;
    let (_, schemes) = project_with(&test, document, &["5", "6"]).await;
    let (first, second) = (schemes[0], schemes[1]);

    let earlier = add_tile(&test.state, first, tile("requirement", "ADV_FSP.4")).await.unwrap().tiles[0].id;
    let later = add_tile(&test.state, second, tile("requirement", "ATE_IND.2")).await.unwrap().tiles[0].id;

    // A requirement of scheme 5 must be met before one of scheme 6.
    let view = add_link(&test.state, second, earlier, later).await.unwrap();
    assert_eq!(view.links[0].meaning, "precedes");
    assert_eq!(view.shadows.len(), 1);
    assert_eq!(view.shadows[0].id, earlier);
    assert_eq!(view.shadows[0].scheme_number, "5");

    // Scheme 5 shows the other end as its shadow.
    let view = open(&test.state, first).await.unwrap();
    assert_eq!(view.shadows[0].id, later);
    assert_eq!(view.schemes.len(), 2);

    // Moving the requirement of 6 into 5: both in one scheme, no shadow.
    let view = move_tiles(&test.state, first, vec![later], first).await.unwrap();
    assert_eq!(view.tiles.len(), 2);
    assert!(view.shadows.is_empty());
    assert_eq!(view.links.len(), 1);

    // "Undo" in either scheme takes the move back (one project).
    let view = undo(&test.state, second).await.unwrap();
    assert_eq!(view.tiles.len(), 1);
    assert_eq!(view.shadows.len(), 1);
}

#[tokio::test]
async fn tiles_of_another_project_are_not_linked() {
    let (test, document) = with_document().await;
    let (_, mine) = project_with(&test, document, &["5"]).await;
    let other = project_actions::create(&test.state, project_form("Other", vec![document])).await.unwrap().project.id;
    let theirs =
        project_actions::create_scheme(&test.state, other, scheme_form("1", "X", None)).await.unwrap().schemes[0].id;

    let a = add_tile(&test.state, mine[0], tile("requirement", "A")).await.unwrap().tiles[0].id;
    let b = add_tile(&test.state, theirs, tile("requirement", "B")).await.unwrap().tiles[0].id;

    assert!(matches!(add_link(&test.state, mine[0], a, b).await, Err(UiError::Invalid(_))));
}

#[tokio::test]
async fn an_analysis_proposes_tiles_through_the_connection() {
    let (test, document) = with_document().await;
    let (_, schemes) = project_with(&test, document, &["5"]).await;
    let scheme = schemes[0];

    // The model answers with this JSON, in pieces.
    let answer = r#"{"tiles": [
        {"key": "a", "kind": "requirement", "identifier": "ADV_FSP.4", "title": "Complete functional specification",
         "quotes": [{"page": 1, "text": "ADV_FSP.4 Complete functional specification"}]},
        {"key": "b", "kind": "input", "title": "Functional specification",
         "quotes": [{"page": 2, "text": "The developer shall provide a functional specification."}]}],
      "links": [{"from": "b", "to": "a"}]}"#;
    let service = fake_model(&test.state, answer).await;

    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&events);

    // No ranges: the pages of the scheme (1 to 3).
    let view = analyse(&test.state, scheme, Vec::new(), 0, move |event| {
        sink.lock().unwrap().push(event);
        true
    })
    .await
    .unwrap();

    assert_eq!(view.tiles.len(), 2);
    assert!(view.tiles.iter().all(|tile| tile.fields.state == "proposed"));
    assert!(view.tiles.iter().all(|tile| tile.fields.sources[0].found));
    assert_eq!(view.links.len(), 1);
    assert!(view.can_undo);

    let events = events.lock().unwrap().clone();
    assert!(matches!(events[0], AnalysisEvent::Part { index: 1, count: 1, .. }));
    assert!(events.iter().any(|event| matches!(event, AnalysisEvent::Applied { added: 2, links: 1, .. })));
    assert!(matches!(events.last(), Some(AnalysisEvent::End { stopped: false })));

    // The chapter and the text of the standard went to the model.
    let request = &service.received()[0];
    let sent = request.body["messages"][1]["content"].as_str().unwrap();
    assert!(sent.contains("The chapter analysed: 5 Chapter."));
    assert!(sent.contains("=== Page 2 ===\nThe developer shall provide a functional specification."));

    // One "Undo" takes the whole analysis back.
    assert!(undo(&test.state, scheme).await.unwrap().tiles.is_empty());
}

#[tokio::test]
async fn several_schemes_are_analysed_one_after_another() {
    let (test, document) = with_document().await;
    let (project, schemes) = project_with(&test, document, &["5", "6"]).await;

    // A scheme without pages is told and left out.
    let bare = project_actions::create_scheme(&test.state, project, scheme_form("7", "No pages", None)).await.unwrap();
    let bare = bare.schemes.last().unwrap().id;

    let answer = r#"{"tiles": [{"key": "a", "kind": "input", "title": "Sample"}], "links": []}"#;
    let service = fake_model(&test.state, answer).await;

    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&events);
    let ids = vec![schemes[0], schemes[1], bare];

    let view = analyse_schemes(&test.state, project, ids, 0, move |event| {
        sink.lock().unwrap().push(event);
        true
    })
    .await
    .unwrap();

    assert_eq!(view.schemes[0].tiles, 1);
    assert_eq!(view.schemes[1].tiles, 1);
    assert_eq!(view.schemes[2].tiles, 0);
    assert_eq!(service.received().len(), 2);

    let events = events.lock().unwrap().clone();
    assert!(matches!(&events[0], AnalysisEvent::Failed { message } if message.contains("7 No pages")));
    assert!(events.iter().any(|event| matches!(event, AnalysisEvent::Scheme { index: 2, count: 2, .. })));

    // One "Undo" takes both back.
    assert!(undo(&test.state, schemes[1]).await.unwrap().tiles.is_empty());
    assert!(open(&test.state, schemes[0]).await.unwrap().tiles.is_empty());
}

#[tokio::test]
async fn the_instructions_are_changed_and_reset() {
    let test = logged_in().await;

    assert_eq!(instructions(&test.state, "chapter".into()).await.unwrap(), DEFAULT_INSTRUCTIONS);
    assert_eq!(save_instructions(&test.state, "chapter".into(), "Mine".into()).await.unwrap(), "Mine");
    assert_eq!(instructions(&test.state, "chapter".into()).await.unwrap(), "Mine");
    assert_eq!(save_instructions(&test.state, "chapter".into(), " ".into()).await.unwrap(), DEFAULT_INSTRUCTIONS);

    assert_eq!(instructions(&test.state, "contents".into()).await.unwrap(), contents::DEFAULT_INSTRUCTIONS);
    assert!(instructions(&test.state, "other".into()).await.is_err());
}
