//! The dialog of the system in which the evaluator chooses a file of
//! evidence (a PDF or a text file). It is opened here, in Rust: the page
//! never sees a path. A dialog waits for the evaluator, so it runs on a
//! thread for blocking work (`spawn_blocking` takes a closure: a library
//! place, rule 9 of CLAUDE.md).

use std::path::PathBuf;

use tauri::{AppHandle, Runtime};
use tauri_plugin_dialog::DialogExt;

use crate::app::error::AppError;

/// The file the evaluator chose; `None` when the dialog was closed.
///
/// Called by: `evidence_add_file` (`evaluation_lab.rs`).
/// Calls: the dialog of the system (`tauri_plugin_dialog`), on a thread for blocking work.
pub async fn choose_evidence<R: Runtime>(app: AppHandle<R>) -> Result<Option<PathBuf>, AppError> {
    let chosen = tokio::task::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Add evidence")
            .add_filter("PDF or text", &["pdf", "txt", "md", "log", "csv", "json", "xml"])
            .blocking_pick_file()
    })
    .await;

    match chosen {
        Ok(Some(file)) => Ok(file.into_path().ok()),
        Ok(None) => Ok(None),
        Err(_) => Err(AppError::Failed("the dialog failed".to_owned())),
    }
}
