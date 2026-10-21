//! The commands the page can call: the whole surface between the
//! page and the program.
//!
//! Each command only hands over to the file of its resource in
//! `commands/`, which can be tested without a window. A command is
//! callable from the page only when it is listed in **four places**
//! (checked by `src/command_lists_tests.rs`):
//!
//! ```text
//! here, or views/**/commands.rs   the function, #[tauri::command]
//! main.rs, generate_handler!      registered with Tauri
//! build.rs, AppManifest           its permission "allow-<name>" made
//! capabilities/default.json       that permission given to the window
//! ```
//!
//! A command missing from the permission list is refused by Tauri
//! when the page calls it: a mistake there closes the door instead
//! of opening it.
//!
//! The arguments arrive from the page as JSON; Tauri turns the
//! names of the page (`chatId`) into the names of Rust (`chat_id`).

pub mod account;
pub mod ai;
pub mod catalogs;
pub mod chats;
pub mod documents;
pub mod oscal;

use tauri::ipc::{Channel, Response};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use super::error::UiError;
use super::serializer::ai_config_view::{AiConfigView, CheckView, ConnectionForm, SavedView};
use super::serializer::app_view::AppView;
use super::serializer::catalog_view::{BuildEvent, CatalogView, PageRange, PlaceForm};
use super::serializer::chat_view::{ChatSummaryView, ChatView, ReplyEvent};
use super::serializer::oscal_view::OscalDocumentView;
use super::serializer::standard_view::{DocumentForm, DocumentView, HitView, LoadEvent};
use super::state::UiState;

/* ---------------------------------------------------------------
 * The program and the account
 * ------------------------------------------------------------- */

/// The state of the program: installed, who is logged in, pages.
#[tauri::command]
pub fn app_state(state: State<'_, UiState>) -> AppView {
    account::app_view(&state)
}

/// The first start.
#[tauri::command]
pub async fn set_up(
    state: State<'_, UiState>,
    name: String,
    password: String,
    password_repeat: String,
    role: String,
    role_confirmed: bool,
) -> Result<AppView, UiError> {
    account::set_up(&state, name, password, password_repeat, role, role_confirmed).await
}

#[tauri::command]
pub async fn log_in(state: State<'_, UiState>, name: String, password: String) -> Result<AppView, UiError> {
    account::log_in(&state, name, password).await
}

#[tauri::command]
pub fn log_out(state: State<'_, UiState>) -> AppView {
    account::log_out(&state)
}

#[tauri::command]
pub async fn change_password(
    state: State<'_, UiState>,
    current: String,
    password: String,
    password_repeat: String,
) -> Result<(), UiError> {
    account::change_password(&state, current, password, password_repeat).await
}

/* ---------------------------------------------------------------
 * Settings, tab AI: the connections to AI services
 * ------------------------------------------------------------- */

#[tauri::command]
pub async fn ai_config(state: State<'_, UiState>) -> Result<AiConfigView, UiError> {
    ai::ai_config(&state).await
}

/// Saves a connection; `id` null: a new one.
#[tauri::command]
pub async fn save_ai_connection(
    state: State<'_, UiState>,
    id: Option<i64>,
    connection: ConnectionForm,
) -> Result<SavedView, UiError> {
    ai::save_connection(&state, id, connection).await
}

#[tauri::command]
pub async fn delete_ai_connection(state: State<'_, UiState>, id: i64) -> Result<AiConfigView, UiError> {
    ai::delete_connection(&state, id).await
}

/// Makes a connection the one the chat uses.
#[tauri::command]
pub async fn activate_ai_connection(state: State<'_, UiState>, id: i64) -> Result<AiConfigView, UiError> {
    ai::activate_connection(&state, id).await
}

#[tauri::command]
pub async fn store_ai_key(state: State<'_, UiState>, id: i64, api_key: String) -> Result<AiConfigView, UiError> {
    ai::store_key(&state, id, api_key).await
}

#[tauri::command]
pub async fn delete_ai_key(state: State<'_, UiState>, id: i64) -> Result<AiConfigView, UiError> {
    ai::delete_key(&state, id).await
}

/// The instruction for a chatbot that helps to fill one field of the
/// form (`connection`, as it is now).
#[tauri::command]
pub fn ai_field_instruction(
    state: State<'_, UiState>,
    field: String,
    connection: ConnectionForm,
) -> Result<String, UiError> {
    ai::field_instruction(&state, &field, connection)
}

/// Sends one short message over a stored connection and reports
/// what happened.
#[tauri::command]
pub async fn check_ai_connection(state: State<'_, UiState>, id: i64) -> Result<CheckView, UiError> {
    ai::check_connection(&state, id).await
}

/* ---------------------------------------------------------------
 * The chat
 * ------------------------------------------------------------- */

#[tauri::command]
pub async fn list_chats(state: State<'_, UiState>) -> Result<Vec<ChatSummaryView>, UiError> {
    chats::list_chats(&state).await
}

#[tauri::command]
pub async fn open_chat(state: State<'_, UiState>, chat_id: i64) -> Result<ChatView, UiError> {
    chats::open_chat(&state, chat_id).await
}

#[tauri::command]
pub async fn delete_chat(state: State<'_, UiState>, chat_id: i64) -> Result<(), UiError> {
    chats::delete_chat(&state, chat_id).await
}

/// Sends a message; the reply arrives through `on_event` while it
/// is written, and the command ends when the reply is stored.
#[tauri::command]
pub async fn send_message(
    state: State<'_, UiState>,
    chat_id: Option<i64>,
    text: String,
    on_event: Channel<ReplyEvent>,
) -> Result<(), UiError> {
    chats::send_message(&state, chat_id, text, move |event| on_event.send(event).is_ok()).await
}

/// Stops the reply being written.
#[tauri::command]
pub fn stop_reply(state: State<'_, UiState>) {
    state.stop_reply();
}

/* ---------------------------------------------------------------
 * The evaluation plan: the documents of standards
 * ------------------------------------------------------------- */

#[tauri::command]
pub async fn list_documents(state: State<'_, UiState>) -> Result<Vec<DocumentView>, UiError> {
    documents::list(&state).await
}

/// Asks for PDFs in a dialog of the system and loads them; the
/// progress arrives through `on_event`. The page never sees a path:
/// the dialog is opened here, in Rust.
#[tauri::command]
pub async fn load_documents(
    app: AppHandle,
    state: State<'_, UiState>,
    on_event: Channel<LoadEvent>,
) -> Result<(), UiError> {
    state.session()?;

    let chosen = tokio::task::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Load documents of a standard")
            .add_filter("PDF", &["pdf", "PDF"])
            .blocking_pick_files()
    })
    .await
    .map_err(|error| UiError::Failed(format!("the dialog failed: {error}")))?;

    let paths: Vec<std::path::PathBuf> =
        chosen.unwrap_or_default().into_iter().filter_map(|file| file.into_path().ok()).collect();

    if paths.is_empty() {
        return Ok(());
    }

    documents::load(&state, paths, move |event| on_event.send(event).is_ok()).await
}

/// Stops the long work running (loading, an analysis).
#[tauri::command]
pub fn stop_work(state: State<'_, UiState>) {
    state.stop_work();
}

#[tauri::command]
pub async fn update_document(
    state: State<'_, UiState>,
    id: i64,
    document: DocumentForm,
) -> Result<DocumentView, UiError> {
    documents::update(&state, id, document).await
}

#[tauri::command]
pub async fn delete_document(state: State<'_, UiState>, id: i64) -> Result<(), UiError> {
    documents::delete(&state, id).await
}

#[tauri::command]
pub async fn document_page(state: State<'_, UiState>, id: i64, page: u32) -> Result<String, UiError> {
    documents::page(&state, id, page).await
}

#[tauri::command]
pub async fn search_documents(
    state: State<'_, UiState>,
    text: String,
    document: Option<i64>,
) -> Result<Vec<HitView>, UiError> {
    documents::search(&state, text, document).await
}

/* ---------------------------------------------------------------
 * The evaluation plan: the documents of OSCAL
 * ------------------------------------------------------------- */

#[tauri::command]
pub async fn list_oscal_documents(state: State<'_, UiState>) -> Result<Vec<OscalDocumentView>, UiError> {
    oscal::list(&state).await
}

/// Asks for a file of OSCAL (JSON) in a dialog of the system and
/// imports it; `None` when no file was chosen.
#[tauri::command]
pub async fn import_oscal_document(
    app: AppHandle,
    state: State<'_, UiState>,
) -> Result<Option<OscalDocumentView>, UiError> {
    state.session()?;

    let chosen = tokio::task::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Import an OSCAL document")
            .add_filter("OSCAL (JSON)", &["json"])
            .blocking_pick_file()
    })
    .await
    .map_err(|error| UiError::Failed(format!("the dialog failed: {error}")))?;

    let Some(path) = chosen.and_then(|file| file.into_path().ok()) else {
        return Ok(None);
    };

    Ok(Some(oscal::import(&state, path).await?))
}

/// Asks where to write document `uuid` in a dialog of the system and
/// writes it; `false` when no place was chosen.
#[tauri::command]
pub async fn export_oscal_document(
    app: AppHandle,
    state: State<'_, UiState>,
    uuid: String,
    file_name: String,
) -> Result<bool, UiError> {
    state.session()?;

    let file_name: String = file_name
        .chars()
        .map(|character| if character.is_alphanumeric() || "-_ .".contains(character) { character } else { '_' })
        .collect();

    let chosen = tokio::task::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Export the OSCAL document")
            .set_file_name(format!("{}.json", file_name.trim()))
            .add_filter("OSCAL (JSON)", &["json"])
            .blocking_save_file()
    })
    .await
    .map_err(|error| UiError::Failed(format!("the dialog failed: {error}")))?;

    let Some(path) = chosen.and_then(|file| file.into_path().ok()) else {
        return Ok(false);
    };

    oscal::export(&state, uuid, &path).await?;

    Ok(true)
}

#[tauri::command]
pub async fn delete_oscal_document(state: State<'_, UiState>, uuid: String) -> Result<(), UiError> {
    oscal::delete(&state, uuid).await
}

/* ---------------------------------------------------------------
 * The evaluation plan: the catalogs of OSCAL
 * ------------------------------------------------------------- */

/// Makes a new catalog from the documents `sources` of a standard.
#[tauri::command]
pub async fn create_catalog(
    state: State<'_, UiState>,
    title: String,
    version: String,
    sources: Vec<i64>,
) -> Result<OscalDocumentView, UiError> {
    catalogs::create(&state, title, version, sources).await
}

#[tauri::command]
pub async fn open_catalog(state: State<'_, UiState>, uuid: String) -> Result<CatalogView, UiError> {
    catalogs::open(&state, uuid).await
}

/// Adds a group (its JSON of OSCAL) at `place`.
#[tauri::command]
pub async fn add_catalog_group(
    state: State<'_, UiState>,
    uuid: String,
    place: PlaceForm,
    group: serde_json::Value,
) -> Result<CatalogView, UiError> {
    catalogs::add_group(&state, uuid, place.into_place()?, group).await
}

#[tauri::command]
pub async fn update_catalog_group(
    state: State<'_, UiState>,
    uuid: String,
    id: String,
    group: serde_json::Value,
) -> Result<CatalogView, UiError> {
    catalogs::update_group(&state, uuid, id, group).await
}

/// Adds a control (its JSON of OSCAL) at `place`.
#[tauri::command]
pub async fn add_catalog_control(
    state: State<'_, UiState>,
    uuid: String,
    place: PlaceForm,
    control: serde_json::Value,
) -> Result<CatalogView, UiError> {
    catalogs::add_control(&state, uuid, place.into_place()?, control).await
}

#[tauri::command]
pub async fn update_catalog_control(
    state: State<'_, UiState>,
    uuid: String,
    id: String,
    control: serde_json::Value,
) -> Result<CatalogView, UiError> {
    catalogs::update_control(&state, uuid, id, control).await
}

#[tauri::command]
pub async fn delete_catalog_object(
    state: State<'_, UiState>,
    uuid: String,
    id: String,
) -> Result<CatalogView, UiError> {
    catalogs::delete_object(&state, uuid, id).await
}

#[tauri::command]
pub async fn add_catalog_quote(
    state: State<'_, UiState>,
    uuid: String,
    object_id: String,
    source: i64,
    page: u32,
    text: String,
) -> Result<CatalogView, UiError> {
    catalogs::add_quote(&state, uuid, object_id, source, page, text).await
}

#[tauri::command]
pub async fn delete_catalog_quote(state: State<'_, UiState>, uuid: String, id: i64) -> Result<CatalogView, UiError> {
    catalogs::delete_quote(&state, uuid, id).await
}

/// Accepts a proposal of the AI, as it is or `corrected`.
#[tauri::command]
pub async fn accept_catalog_proposal(
    state: State<'_, UiState>,
    uuid: String,
    id: i64,
    corrected: Option<serde_json::Value>,
) -> Result<CatalogView, UiError> {
    catalogs::accept_proposal(&state, uuid, id, corrected).await
}

#[tauri::command]
pub async fn reject_catalog_proposal(state: State<'_, UiState>, uuid: String, id: i64) -> Result<CatalogView, UiError> {
    catalogs::reject_proposal(&state, uuid, id).await
}

/// The instructions of a task of the catalogs ("catalog_outline",
/// "catalog_section").
#[tauri::command]
pub async fn catalog_instructions(state: State<'_, UiState>, which: String) -> Result<String, UiError> {
    catalogs::instructions(&state, which).await
}

#[tauri::command]
pub async fn save_catalog_instructions(
    state: State<'_, UiState>,
    which: String,
    text: String,
) -> Result<String, UiError> {
    catalogs::save_instructions(&state, which, text).await
}

/// The AI reads a table of contents and proposes the groups; the
/// progress arrives through `on_event`.
#[tauri::command]
pub async fn propose_catalog_groups(
    state: State<'_, UiState>,
    uuid: String,
    range: PageRange,
    on_event: Channel<BuildEvent>,
) -> Result<CatalogView, UiError> {
    catalogs::propose_groups(&state, uuid, range, move |event| on_event.send(event).is_ok()).await
}

/// The AI reads pages of a group and proposes its controls.
#[tauri::command]
pub async fn propose_catalog_controls(
    state: State<'_, UiState>,
    uuid: String,
    group: Option<String>,
    ranges: Vec<PageRange>,
    part_size: usize,
    on_event: Channel<BuildEvent>,
) -> Result<CatalogView, UiError> {
    catalogs::propose_controls(&state, uuid, group, ranges, part_size, move |event| on_event.send(event).is_ok()).await
}

/// The PDF of a document, as raw bytes (an ArrayBuffer in the page),
/// for the viewer.
#[tauri::command]
pub async fn document_pdf(state: State<'_, UiState>, id: i64) -> Result<Response, UiError> {
    Ok(Response::new(documents::pdf(&state, id).await?))
}
