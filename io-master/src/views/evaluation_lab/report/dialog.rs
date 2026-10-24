//! The dialog of the system in which the evaluator chooses the folder of
//! the report. It is opened here, in Rust: the page never sees a path. A
//! dialog waits for the evaluator, so it runs on a thread for blocking work
//! (`spawn_blocking` takes a closure: a library place, rule 9 of CLAUDE.md).

use std::path::PathBuf;

use tauri::{AppHandle, Runtime};
use tauri_plugin_dialog::DialogExt;

use crate::app::error::AppError;

/// The folder the evaluator chose; `None` when the dialog was closed.
///
/// Called by: `report_export` (`evaluation_lab.rs`).
/// Calls: the dialog of the system (`tauri_plugin_dialog`), on a thread for blocking work.
pub async fn choose_folder<R: Runtime>(app: AppHandle<R>) -> Result<Option<PathBuf>, AppError> {
    let chosen = tokio::task::spawn_blocking(move || {
        app.dialog().file().set_title("The folder of the report").blocking_pick_folder()
    })
    .await;

    match chosen {
        Ok(Some(folder)) => Ok(folder.into_path().ok()),
        Ok(None) => Ok(None),
        Err(_) => Err(AppError::Failed("the dialog failed".to_owned())),
    }
}
