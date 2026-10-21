//! The loading of documents of standards from PDF files: a long work
//! (step 17 of `docs/todo.md`, moved from the window in step 40). Every
//! file is read page by page by Poppler (`pdf_text.rs`), outside the
//! database, then stored with the text of its pages (`create.rs`). A
//! file that fails is told and the next one goes on; "Stop"
//! ([`Cancel`]) ends the work after the page that is read.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::business::work::{Cancel, WorkError, on_db};
use crate::installation::session::Session;
use crate::model::standards::{Document, DocumentFields};

use super::documents;
use super::error::DocumentError;
use super::pdf_text::{self, PdfError, PdfInfo};

/// How a loading goes, for the evaluator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadProgress {
    /// A file starts (counted from 1).
    File { name: String, index: usize, count: usize },
    /// Pages read of the file.
    Pages { done: u32, total: u32 },
    /// The file is stored, as this document.
    Loaded(Document),
    /// The file was not loaded; why.
    Failed { name: String, message: String },
    /// The loading is over.
    End { stopped: bool },
}

/// The name of a file, for the evaluator.
fn file_name(path: &Path) -> String {
    path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default()
}

/// Reads the text of `pdf` page by page, on a thread for blocking work,
/// reporting the pages read. `Ok(None)`: the work was stopped. Also used
/// for the PDFs other files carry.
pub async fn read_pdf(
    pdf: Vec<u8>,
    cancel: &Cancel,
    progress: &(impl Fn(LoadProgress) -> bool + Send + Sync + Clone + 'static),
) -> Result<Option<(Vec<u8>, PdfInfo, Vec<String>)>, String> {
    let (cancel, progress) = (cancel.clone(), progress.clone());

    tokio::task::spawn_blocking(move || {
        let info = pdf_text::info(&pdf).map_err(|error| error.to_string())?;
        let pages = pdf_text::pages(&pdf, info.pages, |done, total| {
            !cancel.is_cancelled() && progress(LoadProgress::Pages { done, total })
        });

        match pages {
            Ok(pages) => Ok(Some((pdf, info, pages))),
            Err(PdfError::Stopped) => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    })
    .await
    .unwrap_or_else(|_| Err(WorkError::Interrupted.to_string()))
}

/// The bytes of the file at `path`, refused when it is larger than a
/// standard may be.
async fn read_file(path: PathBuf) -> Result<Vec<u8>, String> {
    tokio::task::spawn_blocking(move || {
        let size = std::fs::metadata(&path).map_err(|error| error.to_string())?.len();

        if size > pdf_text::MAX_FILE {
            return Err(PdfError::TooLarge.to_string());
        }

        std::fs::read(&path).map_err(|error| error.to_string())
    })
    .await
    .unwrap_or_else(|_| Err(WorkError::Interrupted.to_string()))
}

/// Loads the PDFs at `paths`, one after the other. The title of a
/// document is the title in the file, or its name: a proposal the
/// evaluator corrects.
pub async fn load(
    session: &Arc<Session>,
    paths: Vec<PathBuf>,
    cancel: Cancel,
    progress: impl Fn(LoadProgress) -> bool + Send + Sync + Clone + 'static,
) -> Result<(), WorkError> {
    let count = paths.len();

    let stopped = 'files: {
        for (index, path) in paths.into_iter().enumerate() {
            let name = file_name(&path);

            if cancel.is_cancelled() || !progress(LoadProgress::File { name: name.clone(), index: index + 1, count }) {
                break 'files true;
            }

            let read = match read_file(path.clone()).await {
                Ok(bytes) => read_pdf(bytes, &cancel, &progress).await,
                Err(message) => Err(message),
            };

            let (pdf, info, pages) = match read {
                Ok(Some(read)) => read,
                Ok(None) => break 'files true,
                Err(message) => {
                    progress(LoadProgress::Failed { name, message });
                    continue;
                }
            };

            let fields = DocumentFields {
                title: info.title.unwrap_or_else(|| {
                    path.file_stem().map(|stem| stem.to_string_lossy().into_owned()).unwrap_or_default()
                }),
                part: String::new(),
                version: String::new(),
            };

            let stored_name = name.clone();
            let stored = on_db(session, move |db| {
                let id = match documents::create(db, &fields, &stored_name, &pdf, &pages) {
                    Ok(id) => id,
                    // Which document it is already, so that the evaluator finds it.
                    Err(DocumentError::Duplicate(existing)) => {
                        let title =
                            documents::get(db, existing)?.map(|document| document.fields.title).unwrap_or_default();
                        return Err(WorkError::Invalid(format!(
                            "this file is loaded already, as the document {title}"
                        )));
                    }
                    Err(error) => return Err(error.into()),
                };

                Ok(documents::get(db, id)?)
            })
            .await;

            match stored {
                Ok(Some(document)) => {
                    progress(LoadProgress::Loaded(document));
                }
                Ok(None) => {}
                Err(error) => {
                    progress(LoadProgress::Failed { name, message: error.to_string() });
                }
            }
        }

        false
    };

    progress(LoadProgress::End { stopped });

    Ok(())
}
