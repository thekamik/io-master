//! The command of the connection line.

use tauri::State;

use crate::ai::api::{connections, roles};
use crate::app::error::AppError;
use crate::app::state::AppState;
use crate::model::ai::Role;

use super::view::ConnectionLineView;

/// The connection the chat uses now, or `None` when none is chosen.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn connection_line_get(state: State<'_, AppState>) -> Result<Option<ConnectionLineView>, AppError> {
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

#[cfg(test)]
mod tests {
    //! Unit tests of `connection_line/commands.rs`, through the IPC of Tauri.

    use serde_json::json;

    use crate::ai::api::connections;
    use crate::model::ai::Connection;
    use crate::views::test_support::logged_in_window;

    #[test]
    fn without_a_connection_the_line_is_empty() {
        assert_eq!(logged_in_window().call("connection_line_get", json!({})).unwrap(), json!(null));
    }

    #[test]
    fn the_line_names_the_active_connection_and_its_model() {
        let window = logged_in_window();
        let connection = Connection { name: "Local".to_owned(), model: "llama".to_owned(), ..Connection::template() };
        connections::create(&window.session().db(), &connection).unwrap();

        let line = window.call("connection_line_get", json!({})).unwrap();

        assert_eq!(line, json!({ "name": "Local", "writer": "llama" }));
    }
}
