//! The documents of standards in the tool Create evaluation scheme:
//! loading PDFs, the list, their fields, the text of a page, the
//! search, and the PDF itself for the viewer.
//!
//! Loading is a long work: the text is read in parts
//! (`standards/pdf_text.rs`), every part is reported to the page, and
//! "Stop" ([`UiState::stop_work`]) ends it after the part being read.
//! The files are chosen in a dialog of the system by the command
//! (`commands.rs`); this file gets only their paths.

use std::path::PathBuf;

use crate::model::standards::DocumentFields;
use crate::standards;
use crate::standards::error::DocumentError;
use crate::standards::pdf_text::{self, PdfError};

use super::blocking::blocking;
use super::error::UiError;
use super::serializer::standard_view::{DocumentForm, DocumentView, HitView, LoadEvent};
use super::state::UiState;
use super::user_data::with_user_db;

impl From<DocumentError> for UiError {
    fn from(error: DocumentError) -> UiError {
        match error {
            DocumentError::Db(error) => UiError::Failed(error.to_string()),
            other => UiError::Invalid(other.to_string()),
        }
    }
}

/// The documents, by title and part.
pub async fn list(state: &UiState) -> Result<Vec<DocumentView>, UiError> {
    let documents = with_user_db(state, |db| Ok(standards::read::list(db)?)).await?;

    Ok(documents.into_iter().map(DocumentView::from).collect())
}

/// The name of a file without its folder.
fn file_name(path: &std::path::Path) -> String {
    path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default()
}

/// Loads the PDFs at `paths`, one after the other, reporting to
/// `events` (`false`: the page is gone, the work stops). A file that
/// fails is reported and the next one goes on.
pub async fn load(
    state: &UiState,
    paths: Vec<PathBuf>,
    events: impl Fn(LoadEvent) -> bool + Send + Sync + Clone + 'static,
) -> Result<(), UiError> {
    // Nobody logged in: nothing starts.
    state.session()?;

    let stop = state.start_work();
    let count = paths.len();
    let mut stopped = false;

    for (index, path) in paths.into_iter().enumerate() {
        let name = file_name(&path);

        if *stop.borrow() || !events(LoadEvent::File { name: name.clone(), index: index + 1, count }) {
            stopped = true;
            break;
        }

        // Read the text, outside the database: the slow part.
        let reading = {
            let events = events.clone();
            let stop = stop.clone();
            let path = path.clone();

            blocking(move || {
                let size = std::fs::metadata(&path).map_err(|error| UiError::Invalid(error.to_string()))?.len();

                if size > pdf_text::MAX_FILE {
                    return Err(UiError::Invalid(PdfError::TooLarge.to_string()));
                }

                let bytes = std::fs::read(&path).map_err(|error| UiError::Invalid(error.to_string()))?;
                let info = pdf_text::info(&bytes).map_err(|error| UiError::Invalid(error.to_string()))?;
                let pages = pdf_text::pages(&bytes, info.pages, |done, total| {
                    !*stop.borrow() && events(LoadEvent::Pages { done, total })
                });

                match pages {
                    Ok(pages) => Ok(Some((info, pages, bytes))),
                    Err(PdfError::Stopped) => Ok(None),
                    Err(error) => Err(UiError::Invalid(error.to_string())),
                }
            })
            .await
        };

        let (info, pages, bytes) = match reading {
            Ok(Some(read)) => read,
            Ok(None) => {
                stopped = true;
                break;
            }
            Err(error) => {
                events(LoadEvent::Failed { name, message: error.to_string() });
                continue;
            }
        };

        // The title of the file, or its name, as a proposal the user
        // corrects.
        let fields = DocumentFields {
            title: info.title.unwrap_or_else(|| {
                path.file_stem().map(|stem| stem.to_string_lossy().into_owned()).unwrap_or_default()
            }),
            part: String::new(),
            version: String::new(),
        };

        let stored_name = name.clone();
        let stored = with_user_db(state, move |db| {
            let id = match standards::create::add(db, &fields, &stored_name, &bytes, &pages) {
                Ok(id) => id,
                // Which document it is already, so that the user finds it.
                Err(DocumentError::Duplicate(existing)) => {
                    let title =
                        standards::read::get(db, existing)?.map(|document| document.fields.title).unwrap_or_default();
                    return Err(UiError::Invalid(format!("this file is loaded already, as the document {title}")));
                }
                Err(error) => return Err(error.into()),
            };

            Ok(standards::read::get(db, id)?)
        })
        .await;

        match stored {
            Ok(Some(document)) => {
                events(LoadEvent::Loaded { document: document.into() });
            }
            Ok(None) => {}
            Err(error) => {
                events(LoadEvent::Failed { name, message: error.to_string() });
            }
        }
    }

    events(LoadEvent::End { stopped });

    Ok(())
}

/// Changes the title, part and version of a document.
pub async fn update(state: &UiState, id: i64, form: DocumentForm) -> Result<DocumentView, UiError> {
    let fields: DocumentFields = form.into();

    with_user_db(state, move |db| {
        if !standards::update::update(db, id, &fields)? {
            return Err(UiError::Invalid("the document does not exist".to_owned()));
        }

        Ok(standards::read::get(db, id)?.map(DocumentView::from).expect("the document was just changed"))
    })
    .await
}

/// Deletes a document no scheme uses.
pub async fn delete(state: &UiState, id: i64) -> Result<(), UiError> {
    if with_user_db(state, move |db| Ok(standards::delete::delete(db, id)?)).await? {
        Ok(())
    } else {
        Err(UiError::Invalid("the document does not exist".to_owned()))
    }
}

/// The text of a page.
pub async fn page(state: &UiState, id: i64, number: u32) -> Result<String, UiError> {
    with_user_db(state, move |db| {
        standards::read::page(db, id, number)?.ok_or_else(|| UiError::Invalid(format!("there is no page {number}")))
    })
    .await
}

/// Searches the text of the documents (all, or one).
pub async fn search(state: &UiState, text: String, document: Option<i64>) -> Result<Vec<HitView>, UiError> {
    let hits = with_user_db(state, move |db| Ok(standards::read::search(db, &text, document)?)).await?;

    Ok(hits.into_iter().map(HitView::from).collect())
}

/// The PDF of a document, for the viewer.
pub async fn pdf(state: &UiState, id: i64) -> Result<Vec<u8>, UiError> {
    with_user_db(state, move |db| {
        standards::read::pdf(db, id)?.ok_or_else(|| UiError::Invalid("the document does not exist".to_owned()))
    })
    .await
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "standard_actions_tests.rs"]
mod tests;
