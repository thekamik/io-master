//! Unit tests of `load.rs`, with small PDFs read by the real tools of
//! Poppler and a channel that collects the events.

use super::*;

use std::sync::Mutex;

use serde_json::Value;
use tauri::ipc::InvokeResponseBody;

use crate::app::test_support::{TestState, logged_in};
use crate::test_support::TempDir;
use crate::test_support::standards::pdf;

/// A channel that collects the events it gets, as JSON.
fn collector() -> (Arc<Mutex<Vec<Value>>>, Channel<LoadEvent>) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&events);

    let channel = Channel::new(move |body: InvokeResponseBody| {
        sink.lock().unwrap().push(body.deserialize::<Value>().unwrap());
        Ok(())
    });

    (events, channel)
}

/// Writes PDFs into `directory`; returns their paths.
fn files(directory: &TempDir, files: &[(&str, &[&str])]) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    for (name, pages) in files {
        let path = directory.path().join(name);
        std::fs::write(&path, pdf(pages)).unwrap();
        paths.push(path);
    }

    paths
}

/// The documents stored for the evaluator of `test`.
fn stored(test: &TestState) -> Vec<crate::model::standards::Document> {
    db_standards::read::documents(&test.state.session().unwrap().db()).unwrap()
}

#[tokio::test]
async fn pdfs_are_loaded_with_progress() {
    let test = logged_in().await;
    let directory = TempDir::new();
    let paths = files(&directory, &[("part1.pdf", &["Introduction", "Scope"]), ("part3.pdf", &["ADV_FSP.4"])]);
    let (events, channel) = collector();

    assert!(load(&test.state.session().unwrap(), &paths, &test.state.start_work(), &channel).await);

    let events = events.lock().unwrap().clone();
    assert_eq!(events[0], serde_json::json!({ "event": "file", "name": "part1.pdf", "index": 1, "count": 2 }));
    assert!(events.contains(&serde_json::json!({ "event": "pages", "done": 2, "total": 2 })));
    assert_eq!(events.iter().filter(|event| event["event"] == "loaded").count(), 2);
    assert_eq!(events.last().unwrap(), &serde_json::json!({ "event": "end", "stopped": false }));

    let documents = stored(&test);
    assert_eq!(documents.len(), 2);
    // The title in the file is proposed.
    assert_eq!(documents[0].fields.title, "Test standard");
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
    let (events, channel) = collector();

    load(&test.state.session().unwrap(), &paths, &test.state.start_work(), &channel).await;

    let failed: Vec<Value> =
        events.lock().unwrap().iter().filter(|event| event["event"] == "failed").cloned().collect();
    assert_eq!(failed[0]["name"], "bad.pdf");
    assert_eq!(failed[1]["name"], "good.pdf");
    assert!(failed[1]["message"].as_str().unwrap().contains("loaded already"));
    assert_eq!(stored(&test).len(), 1);
}

#[tokio::test]
async fn stop_ends_the_loading() {
    let test = logged_in().await;
    let directory = TempDir::new();
    let paths = files(&directory, &[("a.pdf", &["A"]), ("b.pdf", &["B"])]);
    let (events, channel) = collector();

    // "Stop" before the work starts: nothing is read.
    let stop = test.state.start_work();
    test.state.stop_work();

    load(&test.state.session().unwrap(), &paths, &stop, &channel).await;

    assert_eq!(events.lock().unwrap().last().unwrap(), &serde_json::json!({ "event": "end", "stopped": true }));
    assert!(stored(&test).is_empty());
}
