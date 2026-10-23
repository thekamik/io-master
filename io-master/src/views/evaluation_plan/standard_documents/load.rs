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
use crate::model::standards::DocumentFields;
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

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "load_tests.rs"]
mod tests;
