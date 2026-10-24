//! The dialogs of the system in which the evaluator chooses a file of OSCAL
//! to import and the place to export a file (a document of OSCAL, a work
//! package). They are opened here, in Rust:
//! the page never sees a path. A dialog waits for the evaluator, so it runs
//! on a thread for blocking work (`spawn_blocking` takes a closure: a
//! library place, rule 9 of CLAUDE.md).

use std::path::PathBuf;

use tauri::{AppHandle, Runtime};
use tauri_plugin_dialog::DialogExt;

use crate::app::error::AppError;

/// The file the evaluator chose; `None` when the dialog was closed.
///
/// Called by: `oscal_documents_import` (`evaluation_plan.rs`).
/// Calls: the dialog of the system (`tauri_plugin_dialog`), on a thread for blocking work.
pub async fn choose_file<R: Runtime>(app: AppHandle<R>) -> Result<Option<PathBuf>, AppError> {
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
        Err(_) => Err(AppError::Failed("the dialog failed".to_owned())),
    }
}

/// The place the evaluator chose for a JSON file named `name` (without
/// its ending), in a dialog titled `title`; `None` when the dialog was
/// closed.
///
/// Called by: `oscal_documents_export`, `work_packages_export` (`evaluation_plan.rs`).
/// Calls: the dialog of the system (`tauri_plugin_dialog`), on a thread for blocking work.
pub async fn choose_place<R: Runtime>(
    app: AppHandle<R>,
    title: &'static str,
    name: String,
) -> Result<Option<PathBuf>, AppError> {
    let chosen = tokio::task::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title(title)
            .set_file_name(format!("{name}.json"))
            .add_filter("JSON", &["json"])
            .blocking_save_file()
    })
    .await;

    match chosen {
        Ok(Some(file)) => Ok(file.into_path().ok()),
        Ok(None) => Ok(None),
        Err(_) => Err(AppError::Failed("the dialog failed".to_owned())),
    }
}
