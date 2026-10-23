//! The commands of the documents of standards.

use std::path::PathBuf;

use tauri::ipc::Channel;
use tauri::{AppHandle, Runtime, State};
use tauri_plugin_dialog::DialogExt;

use crate::app::error::UiError;
use crate::app::state::UiState;
use crate::db::standards as db_standards;

use super::load::{self, MAX_FILES};
use super::stored::{self, DocumentError};
use super::view::{self, DocumentForm, DocumentView, LoadEvent};

/// The documents of the evaluator, by title and part.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn standard_documents_get(state: State<'_, UiState>) -> Result<Vec<DocumentView>, UiError> {
    let session = state.session()?;

    Ok(view::all(db_standards::read::documents(&session.db())?))
}

/// Asks for PDFs in a dialog of the system and loads them (`load.rs`);
/// the events arrive through `on_event`. Nothing chosen: nothing happens.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn standard_documents_load<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, UiState>,
    on_event: Channel<LoadEvent>,
) -> Result<(), UiError> {
    let session = state.session()?;
    let paths = choose_files(app).await?;

    ensure!(paths.len() <= MAX_FILES, UiError::Invalid(format!("at most {MAX_FILES} files can be loaded at once")));

    if paths.is_empty() {
        return Ok(());
    }

    let stop = state.start_work();

    ensure!(
        load::load(&session, &paths, &stop, &on_event).await,
        UiError::Failed("the window can no longer be reached".to_owned())
    );

    Ok(())
}

/// The PDFs the evaluator chose in the dialog of the system; none when
/// the dialog was closed. The dialog waits for the evaluator, so it runs
/// on a thread for blocking work (`spawn_blocking` takes a closure).
async fn choose_files<R: Runtime>(app: AppHandle<R>) -> Result<Vec<PathBuf>, UiError> {
    let chosen = tokio::task::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Load documents of a standard")
            .add_filter("PDF", &["pdf", "PDF"])
            .blocking_pick_files()
    })
    .await;

    let Ok(chosen) = chosen else {
        return Err(UiError::Failed("the dialog failed".to_owned()));
    };

    let mut paths = Vec::new();

    for file in chosen.unwrap_or_default().into_iter().take(MAX_FILES + 1) {
        if let Ok(path) = file.into_path() {
            paths.push(path);
        }
    }

    Ok(paths)
}

/// Stops the loading, if one runs.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn standard_documents_stop(state: State<'_, UiState>) {
    state.stop_work();
}

/// Changes the title, part and version of document `id`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn standard_documents_update(
    state: State<'_, UiState>,
    id: i64,
    document: DocumentForm,
) -> Result<DocumentView, UiError> {
    let session = state.session()?;
    let db = session.db();

    stored::update(&db, id, &document.into_fields())?;

    let Some(document) = db_standards::read::document(&db, id)? else {
        return Err(DocumentError::NotFound.into());
    };

    Ok(DocumentView::new(document))
}

/// Deletes document `id`; refused while an OSCAL document quotes it.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn standard_documents_delete(state: State<'_, UiState>, id: i64) -> Result<(), UiError> {
    let session = state.session()?;

    stored::delete(&session.db(), id)?;

    Ok(())
}

// Unit tests, in their own file to keep this one readable.
#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
