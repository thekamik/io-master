//! The dialog of the system in which the evaluator chooses the PDFs of a
//! standard to load. It is opened here, in Rust: the page never sees a
//! path. The dialog waits for the evaluator, so it runs on a thread for
//! blocking work (`spawn_blocking` takes a closure: a library place, rule 9
//! of CLAUDE.md).

use std::path::PathBuf;

use tauri::{AppHandle, Runtime};
use tauri_plugin_dialog::DialogExt;

use crate::app::error::AppError;

use super::load::MAX_FILES;

/// The PDFs the evaluator chose in the dialog of the system; none when
/// the dialog was closed. The dialog waits for the evaluator, so it runs
/// on a thread for blocking work (`spawn_blocking` takes a closure).
///
/// Called by: `standard_documents_load` (`evaluation_plan.rs`).
/// Calls: the dialog of the system (`tauri_plugin_dialog`), on a thread for blocking work.
pub async fn choose_files<R: Runtime>(app: AppHandle<R>) -> Result<Vec<PathBuf>, AppError> {
    let chosen = tokio::task::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Load documents of a standard")
            .add_filter("PDF", &["pdf", "PDF"])
            .blocking_pick_files()
    })
    .await;

    let Ok(chosen) = chosen else {
        return Err(AppError::Failed("the dialog failed".to_owned()));
    };

    let mut paths = Vec::new();

    for file in chosen.unwrap_or_default().into_iter().take(MAX_FILES + 1) {
        if let Ok(path) = file.into_path() {
            paths.push(path);
        }
    }

    Ok(paths)
}
