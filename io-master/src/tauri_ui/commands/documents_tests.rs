//! Unit tests of `commands/documents.rs`, with small PDFs read by the
//! real tools of Poppler.

use super::*;

use std::sync::{Arc, Mutex};

use crate::business::standards::test_support::pdf as make_pdf;
use crate::tauri_ui::test_support::{installed, logged_in};
use crate::test_support::TempDir;

/// Collects the events of loading.
fn collector() -> (Arc<Mutex<Vec<LoadEvent>>>, impl Fn(LoadEvent) -> bool + Send + Sync + Clone + 'static) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&events);

    (events, move |event| {
        sink.lock().unwrap().push(event);
        true
    })
}

/// Writes PDFs into `directory`; returns their paths.
fn files(directory: &TempDir, files: &[(&str, &[&str])]) -> Vec<PathBuf> {
    files
        .iter()
        .map(|(name, pages)| {
            let path = directory.path().join(name);
            std::fs::write(&path, make_pdf(pages)).unwrap();
            path
        })
        .collect()
}

#[tokio::test]
async fn pdfs_are_loaded_with_progress_and_listed() {
    let test = logged_in().await;
    let directory = TempDir::new();
    let paths = files(&directory, &[("part1.pdf", &["Introduction", "Scope"]), ("part3.pdf", &["ADV_FSP.4"])]);
    let (events, sink) = collector();

    load(&test.state, paths, sink).await.unwrap();

    let events = events.lock().unwrap().clone();
    assert!(matches!(&events[0], LoadEvent::File { name, index: 1, count: 2 } if name == "part1.pdf"));
    assert!(events.iter().any(|event| matches!(event, LoadEvent::Pages { done: 2, total: 2 })));
    assert_eq!(events.iter().filter(|event| matches!(event, LoadEvent::Loaded { .. })).count(), 2);
    assert!(matches!(events.last(), Some(LoadEvent::End { stopped: false })));

    let listed = list(&test.state).await.unwrap();
    assert_eq!(listed.len(), 2);
    // The title of the file is proposed.
    assert_eq!(listed[0].title, "Test standard");

    let first = listed.iter().find(|document| document.file_name == "part1.pdf").unwrap();
    assert_eq!(page(&test.state, first.id, 2).await.unwrap(), "Scope");
    assert!(pdf(&test.state, first.id).await.unwrap().starts_with(b"%PDF"));

    let hits = search(&test.state, "adv_fsp".into(), None).await.unwrap();
    assert_eq!(hits.len(), 1);
}

#[tokio::test]
async fn a_bad_file_and_a_second_copy_are_reported_and_the_rest_goes_on() {
    let test = logged_in().await;
    let directory = TempDir::new();
    let mut paths = files(&directory, &[("good.pdf", &["Text"])]);
    let bad = directory.path().join("bad.pdf");
    std::fs::write(&bad, b"not a pdf").unwrap();
    paths.insert(0, bad);
    paths.push(paths[1].clone());

    let (events, sink) = collector();
    load(&test.state, paths, sink).await.unwrap();

    let events = events.lock().unwrap().clone();
    let failed: Vec<&str> = events
        .iter()
        .filter_map(|event| match event {
            LoadEvent::Failed { name, .. } => Some(name.as_str()),
            _ => None,
        })
        .collect();

    assert_eq!(failed, ["bad.pdf", "good.pdf"]);
    assert_eq!(list(&test.state).await.unwrap().len(), 1);
}

#[tokio::test]
async fn stop_ends_the_loading() {
    let test = Arc::new(logged_in().await);
    let directory = TempDir::new();
    let paths = files(&directory, &[("a.pdf", &["A"]), ("b.pdf", &["B"])]);

    // The page stops the work at the first event.
    let worker = Arc::clone(&test);
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&events);
    load(&test.state, paths, move |event| {
        sink.lock().unwrap().push(event);
        worker.state.stop_work();
        true
    })
    .await
    .unwrap();

    assert!(matches!(events.lock().unwrap().last(), Some(LoadEvent::End { stopped: true })));
    assert!(list(&test.state).await.unwrap().is_empty());
}

#[tokio::test]
async fn a_document_is_renamed_and_deleted() {
    let test = logged_in().await;
    let directory = TempDir::new();
    let (_, sink) = collector();
    load(&test.state, files(&directory, &[("a.pdf", &["A"])]), sink).await.unwrap();
    let id = list(&test.state).await.unwrap()[0].id;

    let form = DocumentForm { title: "Common Criteria".into(), part: "Part 3".into(), version: "CC:2022".into() };
    assert_eq!(update(&test.state, id, form).await.unwrap().part, "Part 3");

    delete(&test.state, id).await.unwrap();
    assert!(list(&test.state).await.unwrap().is_empty());
}

#[tokio::test]
async fn nothing_works_without_a_login() {
    let test = installed();
    let (_, sink) = collector();

    assert!(matches!(load(&test.state, Vec::new(), sink).await, Err(UiError::NotLoggedIn)));
    assert!(matches!(list(&test.state).await, Err(UiError::NotLoggedIn)));
}
