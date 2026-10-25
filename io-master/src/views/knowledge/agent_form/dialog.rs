//! The dialog of the system in which the evaluator chooses where the file
//! of an agent is written. It is opened here, in Rust: the page never sees
//! a path. A dialog waits for the evaluator, so it runs on a thread for
//! blocking work (`spawn_blocking` takes a closure: a library place, rule 9
//! of CLAUDE.md).

use std::path::PathBuf;

use tauri::{AppHandle, Runtime};
use tauri_plugin_dialog::DialogExt;

use crate::app::error::AppError;

/// The place the evaluator chose for the file `name` (without its ending);
/// `None` when the dialog was closed.
///
/// Called by: `agent_form_export` (`knowledge.rs`).
/// Calls: the dialog of the system (`tauri_plugin_dialog`), on a thread for blocking work.
pub async fn choose_place<R: Runtime>(app: AppHandle<R>, name: String) -> Result<Option<PathBuf>, AppError> {
    let chosen = tokio::task::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Export the agent")
            .set_file_name(format!("{name}.iomaster-agent.json"))
            .add_filter("Agent (JSON)", &["json"])
            .blocking_save_file()
    })
    .await;

    match chosen {
        Ok(Some(file)) => Ok(file.into_path().ok()),
        Ok(None) => Ok(None),
        Err(_) => Err(AppError::Failed("the dialog failed".to_owned())),
    }
}
