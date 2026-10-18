//! The documents of OSCAL in the tool Evaluation plan (step 41.2): the
//! list, the import of a file, the export to a file, deleting
//! (`business/oscal/documents.rs`). The files are chosen in a dialog of
//! the system by the command (`commands.rs`); this file gets only their
//! paths.

use std::path::{Path, PathBuf};

use crate::business::oscal::documents;

use crate::tauri_ui::blocking::blocking;
use crate::tauri_ui::error::UiError;
use crate::tauri_ui::serializer::oscal_view::OscalDocumentView;
use crate::tauri_ui::session::with_user_db;
use crate::tauri_ui::state::UiState;

/// The documents, by model and title.
pub async fn list(state: &UiState) -> Result<Vec<OscalDocumentView>, UiError> {
    let documents = with_user_db(state, |db| Ok(documents::list(db)?)).await?;

    Ok(documents.into_iter().map(OscalDocumentView::from).collect())
}

/// Imports the file of OSCAL at `path`.
pub async fn import(state: &UiState, path: PathBuf) -> Result<OscalDocumentView, UiError> {
    let text = blocking(move || {
        std::fs::read_to_string(&path).map_err(|error| UiError::Invalid(format!("the file was not read: {error}")))
    })
    .await?;

    let summary = with_user_db(state, move |db| Ok(documents::import(db, &text)?)).await?;

    Ok(summary.into())
}

/// Writes document `uuid` to the file at `path`.
pub async fn export(state: &UiState, uuid: String, path: &Path) -> Result<(), UiError> {
    let text = with_user_db(state, move |db| Ok(documents::export(db, &uuid)?)).await?;
    let path = path.to_owned();

    blocking(move || {
        std::fs::write(&path, text).map_err(|error| UiError::Failed(format!("the file was not written: {error}")))
    })
    .await
}

/// Deletes document `uuid`.
pub async fn delete(state: &UiState, uuid: String) -> Result<(), UiError> {
    if with_user_db(state, move |db| Ok(documents::delete(db, &uuid)?)).await? {
        Ok(())
    } else {
        Err(UiError::Invalid("the OSCAL document does not exist".to_owned()))
    }
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "oscal_tests.rs"]
mod tests;
