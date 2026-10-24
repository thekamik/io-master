//! The entry of the view Settings: the settings of the program: the
//! connections to AI services and their API keys (tab AI), and what the
//! installation is (tab Program, which shows the state of the program the
//! frame has read: no command of its own); the settings of the user, the
//! account and the encryption, are in `views/users/` (page:
//! `ui/src/views/settings/`, its commands in `ui/src/lib/api/settings.ts`).
//! Which tab and which connection are chosen is presentation, kept by the
//! page. **A key is never sent back to the page**: the page learns only
//! whether one is stored. The flow of data: `docs/architecture.md`,
//! section 12.

use std::sync::Arc;

use tauri::State;

use crate::ai::api::{checks, connections};
use crate::ai::presets::{self, field_help};
use crate::app::error::AppError;
use crate::app::state::AppState;
use crate::db::user::database::UserDb;
use crate::model::ai::Field;
use crate::model::api_key::ApiKey;

use super::api_key::view::ApiKeyView;
use super::connection_check::view::CheckView;
use super::connection_form::view::{ConnectionForm, ConnectionView, NewConnectionView, SavedView};
use super::connection_list::view::ConnectionListView;

// ── When the view opens ───────────────────────────────────────────────

/// The connections of the evaluator by name, and the one the chat uses.
///
/// Called by the page: `ConnectionList.svelte`, when the tab AI opens and when
/// the form changed the connections (`settings.connectionListGet()`).
/// Calls: `connections::list`, `connections::active`, `ConnectionListView::new`.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn connection_list_get(state: State<'_, AppState>) -> Result<ConnectionListView, AppError> {
    let session = state.session()?;
    let db = session.db();

    Ok(ConnectionListView::new(connections::list(&db)?, connections::active(&db)?))
}

/// The template of a new connection, and the presets of well-known
/// services. Reads nothing stored; only for a logged-in evaluator, like
/// everything on the page.
///
/// Called by the page: `ConnectionForm.svelte`, when the tab AI opens
/// (`settings.connectionFormNew()`).
/// Calls: `presets::all`, `NewConnectionView::new`.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn connection_form_new(state: State<'_, AppState>) -> Result<NewConnectionView, AppError> {
    state.session()?;

    Ok(NewConnectionView::new(presets::all()))
}

// ── The actions of the elements ───────────────────────────────────────

/// Connection `id`, without its key.
///
/// Called by the page: `ConnectionForm.svelte`, when a connection is chosen in
/// the list (`settings.connectionFormGet(id)`).
/// Calls: `connections::get`, `ConnectionView::new`.
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
///
/// Called by the page: `ConnectionForm.svelte`, "Save"
/// (`settings.connectionFormSave(...)`).
/// Calls: `ConnectionForm::into_connection` (the fields checked),
/// `connections::create` or `connections::update`.
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
///
/// Called by the page: `ConnectionForm.svelte`, "Delete"
/// (`settings.connectionFormDelete(id)`).
/// Calls: `connections::delete`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn connection_form_delete(state: State<'_, AppState>, id: i64) -> Result<(), AppError> {
    let session = state.session()?;

    ensure!(connections::delete(&session.db(), id)?, no_such_connection());

    Ok(())
}

/// Makes connection `id` the one the chat uses.
///
/// Called by the page: `ConnectionForm.svelte`, "Use in the chat"
/// (`settings.connectionFormActivate(id)`).
/// Calls: `connections::activate`.
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
///
/// Called by the page: `ConnectionForm.svelte`, the help of a field
/// (`settings.connectionFormFieldHelp(...)`).
/// Calls: `Field::from_name`, `ConnectionForm::into_connection`,
/// `field_help::instruction`.
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

/// Whether connection `id` has a key, and whether a header uses it.
///
/// Called by the page: `ApiKey.svelte`, when a connection is chosen and after
/// a save (`settings.apiKeyGet(id)`).
/// Calls: `api_key_view` (below).
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn api_key_get(state: State<'_, AppState>, id: i64) -> Result<ApiKeyView, AppError> {
    let session = state.session()?;

    api_key_view(&session.db(), id)
}

/// Stores the API key of connection `id`, replacing an earlier one. The
/// text is wiped from memory by `ApiKey::new`, whether it is a key or not.
///
/// Called by the page: `ApiKey.svelte`, "Store" (`settings.apiKeyStore(...)`).
/// Calls: `ApiKey::new`, `connections::set_key`, `api_key_view`.
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

    api_key_view(&db, id)
}

/// Deletes the API key of connection `id`.
///
/// Called by the page: `ApiKey.svelte`, "Delete" (`settings.apiKeyDelete(id)`).
/// Calls: `connections::delete_key`, `api_key_view`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn api_key_delete(state: State<'_, AppState>, id: i64) -> Result<ApiKeyView, AppError> {
    let session = state.session()?;
    let db = session.db();

    connections::delete_key(&db, id)?;

    api_key_view(&db, id)
}

/// Checks stored connection `id` with one short message. A connection
/// that cannot be used (no key where one is needed) is reported, not
/// refused. The database is held only while the connection is read,
/// never while the service answers.
///
/// Called by the page: `ConnectionCheck.svelte`, "Check"
/// (`settings.connectionCheckRun(id)`).
/// Calls: `connections::get`, `checks::target` (the model and the key),
/// `checks::create` (one short question, written into the journal of the AI),
/// `CheckView::new`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn connection_check_run(state: State<'_, AppState>, id: i64) -> Result<CheckView, AppError> {
    let session = state.session()?;

    let model = {
        let db = session.db();

        ensure!(connections::get(&db, id)?.is_some(), AppError::Invalid("the connection does not exist".to_owned()));
        checks::target(&db, id)?
    };

    let (report, journal) = checks::create(&session.db_handle(), &Arc::new(model)).await;

    Ok(CheckView::new(report, journal.err()))
}

// ── Helpers of the commands above ─────────────────────────────────────

/// What the page may know of the key of connection `id`.
///
/// Called by: `api_key_get`, `api_key_store`, `api_key_delete`.
/// Calls: `connections::get`, `ApiKeyView::new`.
fn api_key_view(db: &UserDb, id: i64) -> Result<ApiKeyView, AppError> {
    let Some(stored) = connections::get(db, id)? else {
        return Err(AppError::Invalid("the connection does not exist".to_owned()));
    };

    Ok(ApiKeyView::new(&stored))
}

/// The error of a connection that does not exist.
///
/// Called by: `connection_form_get`, `connection_form_delete`,
/// `connection_form_activate`.
fn no_such_connection() -> AppError {
    AppError::Invalid("the connection does not exist".to_owned())
}

#[cfg(test)]
mod tests {
    //! Tests of the commands of the view Settings, called as the page calls
    //! them: by name, through the IPC of Tauri; the check of a connection
    //! with a fake AI service.

    use serde_json::{Value, json};
    use tokio::runtime::Runtime;

    use crate::ai::api::connections;
    use crate::ai::api::test_support::{FakeAnswer, FakeService, completions, completions_events};
    use crate::model::ai::Connection;
    use crate::model::api_key::ApiKey;
    use crate::views::settings::connection_form::view::ConnectionView;
    use crate::views::test_support::{TestWindow, logged_in_window, logged_out_window};

    #[test]
    fn the_connections_are_listed_by_name_with_the_active_one() {
        let window = logged_in_window();
        let named = |name: &str| Connection { name: name.to_owned(), model: "m".to_owned(), ..Connection::template() };
        let first = connections::create(&window.session().db(), &named("B")).unwrap().id;
        let second = connections::create(&window.session().db(), &named("A")).unwrap().id;

        let list = window.call("connection_list_get", json!({})).unwrap();

        assert_eq!(list["connections"], json!([{ "id": second, "name": "A" }, { "id": first, "name": "B" }]));
        assert_eq!(list["active"], first);
    }

    #[test]
    fn the_list_needs_a_login() {
        assert_eq!(logged_out_window().call("connection_list_get", json!({})).unwrap_err()["kind"], "not_logged_in");
    }

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

    /// A window with a connection to `service`, and its id.
    fn with_connection(service: &FakeService) -> (TestWindow, i64) {
        let window = logged_in_window();
        let id = connections::create(&window.session().db(), &completions(service)).unwrap().id;

        (window, id)
    }

    /// The report of the check of connection `id`.
    fn check(window: &TestWindow, id: i64) -> Value {
        window.call("connection_check_run", json!({ "id": id })).unwrap()
    }

    #[test]
    fn a_check_reports_with_the_key_masked() {
        let runtime = Runtime::new().unwrap();
        let service = runtime.block_on(FakeService::start(FakeAnswer::Stream(completions_events(&["O", "K"]))));
        let (window, id) = with_connection(&service);
        connections::set_key(&window.session().db(), id, &ApiKey::new(API_KEY.to_owned()).unwrap()).unwrap();

        let report = check(&window, id);

        assert_eq!(report["works"], true, "{}", report["outcome"]);
        assert_eq!(report["text"], "OK");
        assert_eq!(report["status"], 200);
        assert!(!report.to_string().contains(API_KEY));
        assert_eq!(service.received()[0].header("authorization"), Some(format!("Bearer {API_KEY}").as_str()));
    }

    #[test]
    fn a_check_without_the_needed_key_explains_it() {
        let runtime = Runtime::new().unwrap();
        let service = runtime.block_on(FakeService::start(FakeAnswer::Stream(completions_events(&["OK"]))));
        let (window, id) = with_connection(&service);

        let report = check(&window, id);

        assert_eq!(report["works"], false);
        assert!(report["outcome"].as_str().unwrap().contains("API key"), "{}", report["outcome"]);
        assert!(service.received().is_empty());
    }
}
