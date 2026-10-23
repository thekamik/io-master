//! The commands of the AI of the editor.

use tauri::State;
use tauri::ipc::Channel;

use crate::app::error::AppError;
use crate::app::state::AppState;
use crate::db::standards as db_standards;

use super::view::{self, BuildEvent, PageRange, SourceView};
use super::{controls, groups};

/// The documents the AI can read, by title and part.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn ai_builder_sources(state: State<'_, AppState>) -> Result<Vec<SourceView>, AppError> {
    let session = state.session()?;

    Ok(view::sources(&db_standards::read::documents(&session.db())?))
}

/// The AI reads the table of contents in `range` and proposes the groups
/// of catalog `uuid` (`groups.rs`); the events arrive through `on_event`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn ai_builder_groups(
    state: State<'_, AppState>,
    uuid: String,
    range: PageRange,
    on_event: Channel<BuildEvent>,
) -> Result<(), AppError> {
    let session = state.session()?;
    let stop = state.start_work();

    groups::propose(&session, &uuid, &range, &stop, &on_event).await
}

/// The AI reads the pages `ranges` of group `group` (`None`: the top) and
/// proposes its controls (`controls.rs`), at most `part_size` characters
/// per question; the events arrive through `on_event`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn ai_builder_controls(
    state: State<'_, AppState>,
    uuid: String,
    group: Option<String>,
    ranges: Vec<PageRange>,
    part_size: usize,
    on_event: Channel<BuildEvent>,
) -> Result<(), AppError> {
    let session = state.session()?;
    let stop = state.start_work();

    controls::propose(&session, &uuid, group.as_deref(), &ranges, part_size, &stop, &on_event).await
}

/// Stops the work of the AI, if one runs.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn ai_builder_stop(state: State<'_, AppState>) {
    state.stop_work();
}

// Unit tests, in their own file to keep this one readable.
#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
