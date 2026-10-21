//! The command of the check of a connection.

use std::sync::Arc;

use tauri::State;

use crate::ai::api::{checks, connections};
use crate::tauri_ui::error::UiError;
use crate::tauri_ui::state::UiState;

use super::view::CheckView;

/// Checks stored connection `id` with one short message. A connection
/// that cannot be used (no key where one is needed) is reported, not
/// refused. The database is held only while the connection is read,
/// never while the service answers.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn connection_check_run(state: State<'_, UiState>, id: i64) -> Result<CheckView, UiError> {
    let session = state.session()?;

    let model = {
        let db = session.db();

        ensure!(connections::get(&db, id)?.is_some(), UiError::Invalid("the connection does not exist".to_owned()));
        checks::target(&db, id)?
    };

    let (report, journal) = checks::create(&session.db_handle(), &Arc::new(model)).await;

    Ok(CheckView::new(report, journal.err()))
}

// Unit tests, in their own file to keep this one readable.
#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
