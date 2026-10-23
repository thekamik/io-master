//! The command of the connection line.

use tauri::State;

use crate::ai::api::{connections, roles};
use crate::app::error::UiError;
use crate::app::state::UiState;
use crate::model::ai::Role;

use super::view::ConnectionLineView;

/// The connection the chat uses now, or `None` when none is chosen.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn connection_line_get(state: State<'_, UiState>) -> Result<Option<ConnectionLineView>, UiError> {
    let session = state.session()?;
    let db = session.db();

    let assigned = roles::connection(&db, Role::Consultation)?;
    let id = if assigned.is_some() { assigned } else { connections::active(&db)? };

    let Some(id) = id else {
        return Ok(None);
    };

    let Some(stored) = connections::get(&db, id)? else {
        return Ok(None);
    };

    Ok(Some(ConnectionLineView::new(stored.connection.name, stored.connection.model)))
}

// Unit tests, in their own file to keep this one readable.
#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
