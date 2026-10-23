//! The commands of the documents of OSCAL. The files are chosen in the
//! dialogs of the system, opened here, in Rust: the page never sees a
//! path. A dialog waits for the evaluator, so it runs on a thread for
//! blocking work (`spawn_blocking` takes a closure).

use std::path::PathBuf;

use tauri::{AppHandle, Runtime, State};
use tauri_plugin_dialog::DialogExt;

use crate::app::error::UiError;
use crate::app::state::UiState;
use crate::db::oscal as db_oscal;

use super::file::{self, MAX_FILE};
use super::view::{self, OscalDocumentView};

/// The documents of OSCAL of the evaluator, by model and title.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn oscal_documents_get(state: State<'_, UiState>) -> Result<Vec<OscalDocumentView>, UiError> {
    let session = state.session()?;

    Ok(view::all(db_oscal::read::summaries(&session.db())?))
}

/// Asks for a file of OSCAL (JSON) in a dialog of the system and imports
/// it; `None` when no file was chosen.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn oscal_documents_import<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, UiState>,
) -> Result<Option<OscalDocumentView>, UiError> {
    let session = state.session()?;

    let Some(path) = choose_file(app).await? else {
        return Ok(None);
    };

    let text = read_file(path).await?;
    let summary = file::import(&session.db(), &text)?;

    Ok(Some(OscalDocumentView::new(summary)))
}

/// Asks where to write document `uuid` in a dialog of the system (the
/// name proposed: `file_name`) and writes it; `false` when no place was
/// chosen.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn oscal_documents_export<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, UiState>,
    uuid: String,
    file_name: String,
) -> Result<bool, UiError> {
    let session = state.session()?;
    let text = file::export(&session.db(), &uuid)?;

    let Some(path) = choose_place(app, safe_name(&file_name)).await? else {
        return Ok(false);
    };

    match tokio::fs::write(&path, text).await {
        Ok(()) => Ok(true),
        Err(error) => Err(UiError::Failed(format!("the file was not written: {error}"))),
    }
}

/// Deletes document `uuid` with its evaluation support.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn oscal_documents_delete(state: State<'_, UiState>, uuid: String) -> Result<(), UiError> {
    let session = state.session()?;

    ensure!(file::delete(&session.db(), &uuid)?, UiError::Invalid("the OSCAL document does not exist".to_owned()));

    Ok(())
}

/// The file the evaluator chose; `None` when the dialog was closed.
async fn choose_file<R: Runtime>(app: AppHandle<R>) -> Result<Option<PathBuf>, UiError> {
    let chosen = tokio::task::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Import an OSCAL document")
            .add_filter("OSCAL (JSON)", &["json"])
            .blocking_pick_file()
    })
    .await;

    match chosen {
        Ok(Some(file)) => Ok(file.into_path().ok()),
        Ok(None) => Ok(None),
        Err(_) => Err(UiError::Failed("the dialog failed".to_owned())),
    }
}

/// The place the evaluator chose for a file named `name`; `None` when the
/// dialog was closed.
async fn choose_place<R: Runtime>(app: AppHandle<R>, name: String) -> Result<Option<PathBuf>, UiError> {
    let chosen = tokio::task::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Export the OSCAL document")
            .set_file_name(format!("{name}.json"))
            .add_filter("OSCAL (JSON)", &["json"])
            .blocking_save_file()
    })
    .await;

    match chosen {
        Ok(Some(file)) => Ok(file.into_path().ok()),
        Ok(None) => Ok(None),
        Err(_) => Err(UiError::Failed("the dialog failed".to_owned())),
    }
}

/// The text of the file at `path`, refused when it is larger than
/// [`MAX_FILE`].
async fn read_file(path: PathBuf) -> Result<String, UiError> {
    let size = match tokio::fs::metadata(&path).await {
        Ok(metadata) => metadata.len(),
        Err(error) => return Err(UiError::Invalid(format!("the file was not read: {error}"))),
    };

    ensure!(size <= MAX_FILE, UiError::Invalid(format!("the file is larger than {} MiB", MAX_FILE / 1024 / 1024)));

    match tokio::fs::read_to_string(&path).await {
        Ok(text) => Ok(text),
        Err(error) => Err(UiError::Invalid(format!("the file was not read: {error}"))),
    }
}

/// A name for a file proposed from the title of a document: letters,
/// digits, `-`, `_`, `.` and spaces kept, everything else `_`; at most
/// 100 characters.
fn safe_name(title: &str) -> String {
    let mut name = String::new();

    for character in title.trim().chars().take(100) {
        if character.is_alphanumeric() || "-_ .".contains(character) {
            name.push(character);
        } else {
            name.push('_');
        }
    }

    if name.is_empty() { "oscal".to_owned() } else { name }
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
