//! The commands of the connection form.

use tauri::State;

use crate::ai::api::connections;
use crate::ai::presets::{self, field_help};
use crate::app::error::AppError;
use crate::app::state::AppState;
use crate::model::ai::Field;

use super::view::{ConnectionForm, ConnectionView, NewConnectionView, SavedView};

/// The error of a connection that does not exist.
fn no_such_connection() -> AppError {
    AppError::Invalid("the connection does not exist".to_owned())
}

/// The template of a new connection, and the presets of well-known
/// services. Reads nothing stored; only for a logged-in evaluator, like
/// everything on the page.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn connection_form_new(state: State<'_, AppState>) -> Result<NewConnectionView, AppError> {
    state.session()?;

    Ok(NewConnectionView::new(presets::all()))
}

/// Connection `id`, without its key.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn connection_form_get(state: State<'_, AppState>, id: i64) -> Result<ConnectionView, AppError> {
    let session = state.session()?;

    let Some(stored) = connections::get(&session.db(), id)? else {
        return Err(no_such_connection());
    };

    Ok(ConnectionView::new(Some(stored.id), stored.connection))
}

/// Saves a connection (a new one when `id` is `None`). A field that
/// breaks a rule is named for the page (`AppError::Connection`). A changed
/// address removes the API key (the key is bound to the address it was
/// entered for), and the sentence says so.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn connection_form_save(
    state: State<'_, AppState>,
    id: Option<i64>,
    connection: ConnectionForm,
) -> Result<SavedView, AppError> {
    let session = state.session()?;
    let connection = connection.into_connection()?;

    let saved = match id {
        None => connections::create(&session.db(), &connection)?,
        Some(id) => connections::update(&session.db(), id, &connection)?,
    };

    let notice = if saved.key_removed {
        "The connection was saved. Its API key was removed, because the address changed; enter the key again \
         if the new address needs it."
    } else {
        "The connection was saved."
    };

    Ok(SavedView { id: saved.id, notice })
}

/// Deletes connection `id` with its API key.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn connection_form_delete(state: State<'_, AppState>, id: i64) -> Result<(), AppError> {
    let session = state.session()?;

    ensure!(connections::delete(&session.db(), id)?, no_such_connection());

    Ok(())
}

/// Makes connection `id` the one the chat uses.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn connection_form_activate(state: State<'_, AppState>, id: i64) -> Result<(), AppError> {
    let session = state.session()?;

    ensure!(connections::activate(&session.db(), id)?, no_such_connection());

    Ok(())
}

/// The instruction for a chatbot to help fill `field`, with the form as
/// it is now (saved or not). Reads nothing stored.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn connection_form_field_help(
    state: State<'_, AppState>,
    field: String,
    connection: ConnectionForm,
) -> Result<String, AppError> {
    state.session()?;

    let Some(field) = Field::from_name(&field) else {
        return Err(AppError::Invalid(format!("there is no field {field}")));
    };

    Ok(field_help::instruction(field, &connection.into_connection()?))
}

// Unit tests, in their own file to keep this one readable.
#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
