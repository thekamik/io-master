//! The command of the viewer of a standard.

use tauri::State;
use tauri::ipc::Response;

use crate::app::error::AppError;
use crate::app::state::AppState;
use crate::db::standards as db_standards;

use super::view::{self, ViewerDocumentView};

/// The documents the viewer can show, by title and part.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn pdf_viewer_documents(state: State<'_, AppState>) -> Result<Vec<ViewerDocumentView>, AppError> {
    let session = state.session()?;

    Ok(view::all(db_standards::read::documents(&session.db())?))
}

/// The PDF of document `id`, as raw bytes.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn pdf_viewer_get(state: State<'_, AppState>, id: i64) -> Result<Response, AppError> {
    let session = state.session()?;

    let Some(pdf) = db_standards::read::pdf(&session.db(), id)? else {
        return Err(AppError::Invalid("the document does not exist".to_owned()));
    };

    Ok(Response::new(pdf))
}

// Unit tests, in their own file to keep this one readable.
#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
