//! The commands of the API key.

use tauri::State;

use crate::ai::api::connections;
use crate::app::error::AppError;
use crate::app::state::AppState;
use crate::db::user::database::UserDb;
use crate::model::api_key::ApiKey;

use super::view::ApiKeyView;

/// What the page may know of the key of connection `id`.
fn view(db: &UserDb, id: i64) -> Result<ApiKeyView, AppError> {
    let Some(stored) = connections::get(db, id)? else {
        return Err(AppError::Invalid("the connection does not exist".to_owned()));
    };

    Ok(ApiKeyView::new(&stored))
}

/// Whether connection `id` has a key, and whether a header uses it.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn api_key_get(state: State<'_, AppState>, id: i64) -> Result<ApiKeyView, AppError> {
    let session = state.session()?;

    view(&session.db(), id)
}

/// Stores the API key of connection `id`, replacing an earlier one. The
/// text is wiped from memory by `ApiKey::new`, whether it is a key or not.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn api_key_store(state: State<'_, AppState>, id: i64, api_key: String) -> Result<ApiKeyView, AppError> {
    let session = state.session()?;

    let api_key = match ApiKey::new(api_key) {
        Ok(api_key) => api_key,
        Err(error) => return Err(AppError::Invalid(format!("the key was not stored: {error}"))),
    };

    let db = session.db();

    ensure!(connections::set_key(&db, id, &api_key)?, AppError::Invalid("the connection does not exist".to_owned()));

    view(&db, id)
}

/// Deletes the API key of connection `id`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn api_key_delete(state: State<'_, AppState>, id: i64) -> Result<ApiKeyView, AppError> {
    let session = state.session()?;
    let db = session.db();

    connections::delete_key(&db, id)?;

    view(&db, id)
}

#[cfg(test)]
mod tests {
    //! Unit tests of `api_key/commands.rs`, through the IPC of Tauri.

    use serde_json::json;

    use crate::ai::api::connections;
    use crate::model::ai::Connection;
    use crate::views::test_support::logged_in_window;

    const API_KEY: &str = "sk-secret-of-anna";

    #[test]
    fn the_key_is_stored_and_never_given_back() {
        let window = logged_in_window();
        let connection = Connection {
            name: "Service".to_owned(),
            model: "m".to_owned(),
            headers: "authorization: Bearer {{api_key}}".to_owned(),
            ..Connection::template()
        };
        let id = connections::create(&window.session().db(), &connection).unwrap().id;

        let before = window.call("api_key_get", json!({ "id": id })).unwrap();
        assert_eq!(before, json!({ "has_key": false, "key_unused": false, "header_uses_key": true }));

        let stored = window.call("api_key_store", json!({ "id": id, "apiKey": format!("  {API_KEY}  ") })).unwrap();
        assert_eq!(stored["has_key"], true);
        assert!(!stored.to_string().contains(API_KEY));

        let deleted = window.call("api_key_delete", json!({ "id": id })).unwrap();
        assert_eq!(deleted["has_key"], false);
    }

    #[test]
    fn a_key_no_header_uses_is_reported() {
        let window = logged_in_window();
        let connection = Connection { name: "Local".to_owned(), model: "m".to_owned(), ..Connection::template() };
        let id = connections::create(&window.session().db(), &connection).unwrap().id;

        let stored = window.call("api_key_store", json!({ "id": id, "apiKey": API_KEY })).unwrap();

        assert_eq!(stored["key_unused"], true);
    }

    #[test]
    fn a_key_of_no_connection_is_refused() {
        let window = logged_in_window();

        assert_eq!(window.call("api_key_store", json!({ "id": 9, "apiKey": API_KEY })).unwrap_err()["kind"], "invalid");
        assert_eq!(window.call("api_key_get", json!({ "id": 9 })).unwrap_err()["kind"], "invalid");
    }
}
