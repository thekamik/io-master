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

#[cfg(test)]
mod tests {
    //! Unit tests of `connection_form/commands.rs`, through the IPC of Tauri.

    use serde_json::{Value, json};

    use crate::model::ai::Connection;
    use crate::views::settings::connection_form::view::ConnectionView;
    use crate::views::test_support::{TestWindow, logged_in_window, logged_out_window};

    /// The form the page sends for `connection`: the fields of the view
    /// without `id`.
    fn form(connection: Connection) -> Value {
        let mut fields = serde_json::to_value(ConnectionView::new(None, connection)).unwrap();
        fields.as_object_mut().unwrap().remove("id");
        fields
    }

    /// A connection named `name` that needs nothing to be saved.
    fn named(name: &str) -> Connection {
        // Nothing listens on this port: saving does not need the service.
        Connection {
            name: name.to_owned(),
            model: "llama".to_owned(),
            url: "http://127.0.0.1:9/v1/chat/completions".to_owned(),
            ..Connection::template()
        }
    }

    fn save(window: &TestWindow, id: Option<i64>, connection: Value) -> Result<Value, Value> {
        window.call("connection_form_save", json!({ "id": id, "connection": connection }))
    }

    #[test]
    fn a_connection_is_saved_read_and_changed() {
        let window = logged_in_window();

        let saved = save(&window, None, form(named("Ollama"))).unwrap();
        assert_eq!(saved["notice"], "The connection was saved.");
        let id = saved["id"].as_i64().unwrap();

        let read = window.call("connection_form_get", json!({ "id": id })).unwrap();
        assert_eq!(read["id"], id);
        assert_eq!(read["name"], "Ollama");
        assert_eq!(read["answer"], "stream");

        save(&window, Some(id), form(named("Renamed"))).unwrap();
        assert_eq!(window.call("connection_form_get", json!({ "id": id })).unwrap()["name"], "Renamed");

        // The first connection is the active one.
        assert_eq!(window.call("connection_list_get", json!({})).unwrap()["active"], id);
    }

    #[test]
    fn a_broken_field_is_named_for_the_page() {
        let window = logged_in_window();
        let outside = Connection { url: "http://10.0.0.5/v1".to_owned(), ..named("Lab") };

        let error = save(&window, None, form(outside)).unwrap_err();

        assert_eq!(error["kind"], "invalid");
        assert_eq!(error["field"], "url");
    }

    #[test]
    fn an_unknown_answer_mode_or_field_is_refused() {
        let window = logged_in_window();

        let mut other = form(named("X"));
        other["answer"] = json!("other");
        assert_eq!(save(&window, None, other).unwrap_err()["kind"], "invalid");

        let mut extra = form(named("X"));
        extra["api_key"] = json!("sk-should-not-be-here");
        assert!(save(&window, None, extra).is_err());
    }

    #[test]
    fn the_active_connection_is_chosen_and_deleted() {
        let window = logged_in_window();
        let first = save(&window, None, form(named("A"))).unwrap()["id"].clone();
        let second = save(&window, None, form(named("B"))).unwrap()["id"].clone();

        window.call("connection_form_activate", json!({ "id": second })).unwrap();
        assert_eq!(window.call("connection_list_get", json!({})).unwrap()["active"], second);
        assert_eq!(window.call("connection_form_activate", json!({ "id": 99 })).unwrap_err()["kind"], "invalid");

        window.call("connection_form_delete", json!({ "id": second })).unwrap();

        let list = window.call("connection_list_get", json!({})).unwrap();
        assert_eq!(list["active"], json!(null));
        assert_eq!(list["connections"], json!([{ "id": first, "name": "A" }]));
        assert_eq!(window.call("connection_form_delete", json!({ "id": second })).unwrap_err()["kind"], "invalid");
    }

    #[test]
    fn a_new_connection_starts_from_the_template_and_the_presets() {
        let new = logged_in_window().call("connection_form_new", json!({})).unwrap();

        assert_eq!(new["template"]["id"], json!(null));
        assert_eq!(new["presets"][0]["id"], "local-gemma");
        assert_eq!(new["presets"][0]["fields"]["id"], json!(null));
    }

    #[test]
    fn an_instruction_is_given_for_a_field_of_the_form() {
        let window = logged_in_window();
        let ask = |field: &str| {
            window.call("connection_form_field_help", json!({ "field": field, "connection": form(named("X")) }))
        };

        assert!(ask("text_path").unwrap().as_str().unwrap().contains("The field: Path to the text"));
        assert_eq!(ask("api_key").unwrap_err()["kind"], "invalid");
    }

    #[test]
    fn nothing_works_without_a_login() {
        let window = logged_out_window();

        assert_eq!(window.call("connection_form_new", json!({})).unwrap_err()["kind"], "not_logged_in");
        assert_eq!(save(&window, None, form(named("X"))).unwrap_err()["kind"], "not_logged_in");
    }
}
