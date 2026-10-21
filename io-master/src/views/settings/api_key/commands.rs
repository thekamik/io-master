//! The commands of the API key.

use tauri::State;

use crate::ai::api::connections;
use crate::db::user::database::UserDb;
use crate::model::api_key::ApiKey;
use crate::tauri_ui::error::UiError;
use crate::tauri_ui::state::UiState;

use super::view::ApiKeyView;

/// What the page may know of the key of connection `id`.
fn view(db: &UserDb, id: i64) -> Result<ApiKeyView, UiError> {
    let Some(stored) = connections::get(db, id)? else {
        return Err(UiError::Invalid("the connection does not exist".to_owned()));
    };

    Ok(ApiKeyView::new(&stored))
}

/// Whether connection `id` has a key, and whether a header uses it.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn api_key_get(state: State<'_, UiState>, id: i64) -> Result<ApiKeyView, UiError> {
    let session = state.session()?;

    view(&session.db(), id)
}

/// Stores the API key of connection `id`, replacing an earlier one. The
/// text is wiped from memory by `ApiKey::new`, whether it is a key or not.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn api_key_store(state: State<'_, UiState>, id: i64, api_key: String) -> Result<ApiKeyView, UiError> {
    let session = state.session()?;

    let api_key = match ApiKey::new(api_key) {
        Ok(api_key) => api_key,
        Err(error) => return Err(UiError::Invalid(format!("the key was not stored: {error}"))),
    };

    let db = session.db();

    ensure!(connections::set_key(&db, id, &api_key)?, UiError::Invalid("the connection does not exist".to_owned()));

    view(&db, id)
}

/// Deletes the API key of connection `id`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn api_key_delete(state: State<'_, UiState>, id: i64) -> Result<ApiKeyView, UiError> {
    let session = state.session()?;
    let db = session.db();

    connections::delete_key(&db, id)?;

    view(&db, id)
}

// Unit tests, in their own file to keep this one readable.
#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
