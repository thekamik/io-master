//! The documents of standards in the tool Evaluation plan:
//! loading PDFs, the list, their fields, the text of a page, the
//! search, and the PDF itself for the viewer.
//!
//! Loading is a long work: the text is read in parts
//! (`business/standards/pdf_text.rs`), every part is reported to the page, and
//! "Stop" ([`UiState::stop_work`]) ends it after the part being read.
//! The files are chosen in a dialog of the system by the command
//! (`commands.rs`); this file gets only their paths.

use std::path::PathBuf;

use crate::business::standards;
use crate::business::standards::load::{self, LoadProgress};
use crate::business::work::Cancel;
use crate::model::standards::DocumentFields;

use crate::tauri_ui::error::UiError;
use crate::tauri_ui::serializer::standard_view::{DocumentForm, DocumentView, HitView, LoadEvent};
use crate::tauri_ui::session::with_user_db;
use crate::tauri_ui::state::UiState;

/// The documents, by title and part.
pub async fn list(state: &UiState) -> Result<Vec<DocumentView>, UiError> {
    let documents = with_user_db(state, |db| Ok(standards::documents::list(db)?)).await?;

    Ok(documents.into_iter().map(DocumentView::from).collect())
}

/// Loads the PDFs at `paths`, one after the other
/// (`business/standards/load.rs`), reporting to `events` (`false`: the
/// page is gone, the work stops). A file that fails is reported and the
/// next one goes on.
pub async fn load(
    state: &UiState,
    paths: Vec<PathBuf>,
    events: impl Fn(LoadEvent) -> bool + Send + Sync + Clone + 'static,
) -> Result<(), UiError> {
    let session = state.session()?;
    let cancel = Cancel::new(state.start_work());

    load::load(&session, paths, cancel, move |progress: LoadProgress| events(progress.into())).await?;

    Ok(())
}

/// Changes the title, part and version of a document.
pub async fn update(state: &UiState, id: i64, form: DocumentForm) -> Result<DocumentView, UiError> {
    let fields: DocumentFields = form.into();

    with_user_db(state, move |db| {
        if !standards::documents::update(db, id, &fields)? {
            return Err(UiError::Invalid("the document does not exist".to_owned()));
        }

        Ok(standards::documents::get(db, id)?.map(DocumentView::from).expect("the document was just changed"))
    })
    .await
}

/// Deletes a document.
pub async fn delete(state: &UiState, id: i64) -> Result<(), UiError> {
    if with_user_db(state, move |db| Ok(standards::documents::delete(db, id)?)).await? {
        Ok(())
    } else {
        Err(UiError::Invalid("the document does not exist".to_owned()))
    }
}

/// The text of a page.
pub async fn page(state: &UiState, id: i64, number: u32) -> Result<String, UiError> {
    with_user_db(state, move |db| {
        standards::documents::page(db, id, number)?
            .ok_or_else(|| UiError::Invalid(format!("there is no page {number}")))
    })
    .await
}

/// Searches the text of the documents (all, or one).
pub async fn search(state: &UiState, text: String, document: Option<i64>) -> Result<Vec<HitView>, UiError> {
    let hits = with_user_db(state, move |db| Ok(standards::documents::search(db, &text, document)?)).await?;

    Ok(hits.into_iter().map(HitView::from).collect())
}

/// The PDF of a document, for the viewer.
pub async fn pdf(state: &UiState, id: i64) -> Result<Vec<u8>, UiError> {
    with_user_db(state, move |db| {
        standards::documents::pdf(db, id)?.ok_or_else(|| UiError::Invalid("the document does not exist".to_owned()))
    })
    .await
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "documents_tests.rs"]
mod tests;
