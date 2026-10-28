//! The loading of documents of standards from PDF files: a long work.
//! Every file is read part by part by Poppler (`pdf/poppler.rs`), outside
//! the database, then stored with the text of its pages (`stored.rs`). A
//! file that fails is told and the next one goes on; "Stop" ends the work
//! after the part that is read. The events go to the page through a
//! channel of Tauri as they happen (`view.rs`, `LoadEvent`):
//!
//! ```text
//! for every file:    File ─► (Pages, after every part of 20 pages) ─► Loaded or Failed
//! at the end:        End { stopped }
//! ```
//!
//! The PDF is read on threads for blocking work (`spawn_blocking`, which
//! takes a closure: the one place here where a library needs one). The
//! database is held only while a document is stored.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tauri::ipc::Channel;
use tokio::sync::watch;

use crate::db::standards as db_standards;
use crate::installation::session::Session;
use crate::model_old::standards::DocumentFields;
use crate::pdf::poppler::{self, MAX_FILE, MAX_PAGES, PART, PdfError, PdfInfo};

use super::stored::{self, DocumentError};
use super::view::{DocumentView, LoadEvent};

/// The most files loaded at once.
pub const MAX_FILES: usize = 100;

/// What became of one file.
enum Outcome {
    Loaded(DocumentView),
    Failed(String),
    Stopped,
}

/// Loads the PDFs at `paths`, one after the other, sending the events to
/// `events`; `stop` set to `true` (the "Stop" of the page, or the logout)
/// ends the work after the part being read, as does a page that can no
/// longer be reached. Returns whether the page got the last event.
pub async fn load(
    session: &Session,
    paths: &[PathBuf],
    stop: &watch::Receiver<bool>,
    events: &Channel<LoadEvent>,
) -> bool {
    let mut stopped = paths.len() > MAX_FILES;

    for (index, path) in paths.iter().take(MAX_FILES).enumerate() {
        let name = file_name(path);
        let started = LoadEvent::File { name: name.clone(), index: index + 1, count: paths.len() };

        if *stop.borrow() || events.send(started).is_err() {
            stopped = true;
            break;
        }

        let event = match load_file(session, path, stop, events).await {
            Outcome::Loaded(document) => LoadEvent::Loaded { document },
            Outcome::Failed(message) => LoadEvent::Failed { name, message },
            Outcome::Stopped => {
                stopped = true;
                break;
            }
        };

        if events.send(event).is_err() {
            stopped = true;
            break;
        }
    }

    events.send(LoadEvent::End { stopped }).is_ok()
}

/// Reads and stores the file at `path`.
async fn load_file(
    session: &Session,
    path: &Path,
    stop: &watch::Receiver<bool>,
    events: &Channel<LoadEvent>,
) -> Outcome {
    let pdf = match read_file(path.to_owned()).await {
        Ok(pdf) => Arc::new(pdf),
        Err(message) => return Outcome::Failed(message),
    };

    let info = match read_info(Arc::clone(&pdf)).await {
        Ok(info) => info,
        Err(message) => return Outcome::Failed(message),
    };

    let pages = match read_pages(&pdf, info.pages, stop, events).await {
        Ok(Some(pages)) => pages,
        Ok(None) => return Outcome::Stopped,
        Err(message) => return Outcome::Failed(message),
    };

    // The title of the file, or its name: a proposal the evaluator corrects.
    let title = match info.title {
        Some(title) => title,
        None => file_stem(path),
    };

    let fields = DocumentFields { title, part: String::new(), version: String::new() };

    match store(session, &fields, &file_name(path), &pdf, &pages) {
        Ok(document) => Outcome::Loaded(document),
        Err(message) => Outcome::Failed(message),
    }
}

/// Stores a read file and gives the document back as the page shows it.
fn store(
    session: &Session,
    fields: &DocumentFields,
    name: &str,
    pdf: &[u8],
    pages: &[String],
) -> Result<DocumentView, String> {
    let db = session.db();

    let id = match stored::create(&db, fields, name, pdf, pages) {
        Ok(id) => id,
        // Which document it is already, so that the evaluator finds it.
        Err(DocumentError::Duplicate(existing)) => {
            let title = match db_standards::read::document(&db, existing) {
                Ok(Some(document)) => document.fields.title,
                Ok(None) | Err(_) => String::new(),
            };

            return Err(format!("this file is loaded already, as the document {title}"));
        }
        Err(error) => return Err(error.to_string()),
    };

    match db_standards::read::document(&db, id) {
        Ok(Some(document)) => Ok(DocumentView::new(document)),
        Ok(None) => Err("the document was stored but cannot be read".to_owned()),
        Err(error) => Err(error.to_string()),
    }
}

/// The text of the `total` pages of `pdf`, part by part, with a `Pages`
/// event after every part. `Ok(None)`: the work was stopped.
async fn read_pages(
    pdf: &Arc<Vec<u8>>,
    total: u32,
    stop: &watch::Receiver<bool>,
    events: &Channel<LoadEvent>,
) -> Result<Option<Vec<String>>, String> {
    let mut pages = Vec::new();
    let mut first = 1;

    for _ in 0..=MAX_PAGES / PART {
        if first > total {
            break;
        }

        if *stop.borrow() {
            return Ok(None);
        }

        let last = (first + PART - 1).min(total);
        let shared = Arc::clone(pdf);
        let read = tokio::task::spawn_blocking(move || poppler::text_of_pages(&shared, first, last)).await;

        match read {
            Ok(Ok(part)) => pages.extend(part),
            Ok(Err(error)) => return Err(error.to_string()),
            Err(_) => return Err("the reading was interrupted".to_owned()),
        }

        if events.send(LoadEvent::Pages { done: last, total }).is_err() {
            return Ok(None);
        }

        first = last + 1;
    }

    Ok(Some(pages))
}

/// The number of pages and the title of `pdf`.
async fn read_info(pdf: Arc<Vec<u8>>) -> Result<PdfInfo, String> {
    match tokio::task::spawn_blocking(move || poppler::info(&pdf)).await {
        Ok(Ok(info)) => Ok(info),
        Ok(Err(error)) => Err(error.to_string()),
        Err(_) => Err("the reading was interrupted".to_owned()),
    }
}

/// The bytes of the file at `path`, on a thread for blocking work.
async fn read_file(path: PathBuf) -> Result<Vec<u8>, String> {
    match tokio::task::spawn_blocking(move || read_bounded(&path)).await {
        Ok(read) => read,
        Err(_) => Err("the reading was interrupted".to_owned()),
    }
}

/// The bytes of the file at `path`, refused when it is larger than a
/// standard may be.
fn read_bounded(path: &Path) -> Result<Vec<u8>, String> {
    let size = match std::fs::metadata(path) {
        Ok(metadata) => metadata.len(),
        Err(error) => return Err(error.to_string()),
    };

    ensure!(size <= MAX_FILE, PdfError::TooLarge.to_string());

    match std::fs::read(path) {
        Ok(bytes) => Ok(bytes),
        Err(error) => Err(error.to_string()),
    }
}

/// The name of a file, for the evaluator.
fn file_name(path: &Path) -> String {
    match path.file_name() {
        Some(name) => name.to_string_lossy().into_owned(),
        None => String::new(),
    }
}

/// The name of a file without its extension.
fn file_stem(path: &Path) -> String {
    match path.file_stem() {
        Some(stem) => stem.to_string_lossy().into_owned(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
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
    fn stored(test: &TestState) -> Vec<crate::model_old::standards::Document> {
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
}
