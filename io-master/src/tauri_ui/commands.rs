//! The commands the page can call: the whole surface between the
//! page and the program.
//!
//! Each command only hands over to an action (`*_actions.rs`), which
//! does the work and can be tested without a window. A command is
//! callable from the page only when it is listed in **four places**:
//!
//! ```text
//! here                            the function, #[tauri::command]
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

use tauri::ipc::{Channel, Response};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use super::account_actions;
use super::ai_config_actions;
use super::chat_actions;
use super::error::UiError;
use super::scan_actions;
use super::serializer::ai_config_view::{AiConfigView, CheckView, ConnectionForm, SavedView};
use super::serializer::app_view::AppView;
use super::serializer::chat_view::{ChatSummaryView, ChatView, ReplyEvent};
use super::project_actions;
use super::scheme_actions;
use super::serializer::project_view::{ProjectDataView, ProjectForm, ProjectView, SchemeForm};
use super::serializer::scan_view::ScanView;
use super::serializer::scheme_view::{AnalysisEvent, PageRange, SchemeDataView, TileForm};
use super::serializer::standard_view::{DocumentForm, DocumentView, HitView, LoadEvent};
use super::standard_actions;
use super::state::UiState;

/* ---------------------------------------------------------------
 * The program and the account
 * ------------------------------------------------------------- */

/// The state of the program: installed, who is logged in, pages.
#[tauri::command]
pub fn app_state(state: State<'_, UiState>) -> AppView {
    account_actions::app_view(&state)
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
    account_actions::set_up(&state, name, password, password_repeat, role, role_confirmed).await
}

#[tauri::command]
pub async fn log_in(state: State<'_, UiState>, name: String, password: String) -> Result<AppView, UiError> {
    account_actions::log_in(&state, name, password).await
}

#[tauri::command]
pub fn log_out(state: State<'_, UiState>) -> AppView {
    account_actions::log_out(&state)
}

#[tauri::command]
pub async fn change_password(
    state: State<'_, UiState>,
    current: String,
    password: String,
    password_repeat: String,
) -> Result<(), UiError> {
    account_actions::change_password(&state, current, password, password_repeat).await
}

/* ---------------------------------------------------------------
 * Settings, tab AI: the connections to AI services
 * ------------------------------------------------------------- */

#[tauri::command]
pub async fn ai_config(state: State<'_, UiState>) -> Result<AiConfigView, UiError> {
    ai_config_actions::ai_config(&state).await
}

/// Saves a connection; `id` null: a new one.
#[tauri::command]
pub async fn save_ai_connection(
    state: State<'_, UiState>,
    id: Option<i64>,
    connection: ConnectionForm,
) -> Result<SavedView, UiError> {
    ai_config_actions::save_connection(&state, id, connection).await
}

#[tauri::command]
pub async fn delete_ai_connection(state: State<'_, UiState>, id: i64) -> Result<AiConfigView, UiError> {
    ai_config_actions::delete_connection(&state, id).await
}

/// Makes a connection the one the chat uses.
#[tauri::command]
pub async fn activate_ai_connection(state: State<'_, UiState>, id: i64) -> Result<AiConfigView, UiError> {
    ai_config_actions::activate_connection(&state, id).await
}

#[tauri::command]
pub async fn store_ai_key(state: State<'_, UiState>, id: i64, api_key: String) -> Result<AiConfigView, UiError> {
    ai_config_actions::store_key(&state, id, api_key).await
}

#[tauri::command]
pub async fn delete_ai_key(state: State<'_, UiState>, id: i64) -> Result<AiConfigView, UiError> {
    ai_config_actions::delete_key(&state, id).await
}

/// The instruction for a chatbot that helps to fill one field of the
/// form (`connection`, as it is now).
#[tauri::command]
pub fn ai_field_instruction(state: State<'_, UiState>, field: String, connection: ConnectionForm) -> Result<String, UiError> {
    ai_config_actions::field_instruction(&state, &field, connection)
}

/// Sends one short message over a stored connection and reports
/// what happened.
#[tauri::command]
pub async fn check_ai_connection(state: State<'_, UiState>, id: i64) -> Result<CheckView, UiError> {
    ai_config_actions::check_connection(&state, id).await
}

/* ---------------------------------------------------------------
 * The scan
 * ------------------------------------------------------------- */

#[tauri::command]
pub async fn scan_ports(state: State<'_, UiState>) -> Result<ScanView, UiError> {
    scan_actions::scan(&state).await
}

#[tauri::command]
pub async fn save_scan_settings(
    state: State<'_, UiState>,
    all_types: bool,
    types: Vec<String>,
    columns: Vec<String>,
) -> Result<ScanView, UiError> {
    scan_actions::save_scan_settings(&state, all_types, types, columns).await
}

/* ---------------------------------------------------------------
 * The chat
 * ------------------------------------------------------------- */

#[tauri::command]
pub async fn list_chats(state: State<'_, UiState>) -> Result<Vec<ChatSummaryView>, UiError> {
    chat_actions::list_chats(&state).await
}

#[tauri::command]
pub async fn open_chat(state: State<'_, UiState>, chat_id: i64) -> Result<ChatView, UiError> {
    chat_actions::open_chat(&state, chat_id).await
}

#[tauri::command]
pub async fn delete_chat(state: State<'_, UiState>, chat_id: i64) -> Result<(), UiError> {
    chat_actions::delete_chat(&state, chat_id).await
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
    chat_actions::send_message(&state, chat_id, text, move |event| on_event.send(event).is_ok()).await
}

/// Stops the reply being written.
#[tauri::command]
pub fn stop_reply(state: State<'_, UiState>) {
    state.stop_reply();
}

/* ---------------------------------------------------------------
 * Create evaluation scheme: the documents of standards
 * ------------------------------------------------------------- */

#[tauri::command]
pub async fn list_documents(state: State<'_, UiState>) -> Result<Vec<DocumentView>, UiError> {
    standard_actions::list(&state).await
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

    let paths: Vec<std::path::PathBuf> = chosen
        .unwrap_or_default()
        .into_iter()
        .filter_map(|file| file.into_path().ok())
        .collect();

    if paths.is_empty() {
        return Ok(());
    }

    standard_actions::load(&state, paths, move |event| on_event.send(event).is_ok()).await
}

/// Stops the long work running (loading, an analysis).
#[tauri::command]
pub fn stop_work(state: State<'_, UiState>) {
    state.stop_work();
}

#[tauri::command]
pub async fn update_document(state: State<'_, UiState>, id: i64, document: DocumentForm) -> Result<DocumentView, UiError> {
    standard_actions::update(&state, id, document).await
}

#[tauri::command]
pub async fn delete_document(state: State<'_, UiState>, id: i64) -> Result<(), UiError> {
    standard_actions::delete(&state, id).await
}

#[tauri::command]
pub async fn document_page(state: State<'_, UiState>, id: i64, page: u32) -> Result<String, UiError> {
    standard_actions::page(&state, id, page).await
}

#[tauri::command]
pub async fn search_documents(
    state: State<'_, UiState>,
    text: String,
    document: Option<i64>,
) -> Result<Vec<HitView>, UiError> {
    standard_actions::search(&state, text, document).await
}

/* ---------------------------------------------------------------
 * Create evaluation scheme: the test projects
 * ------------------------------------------------------------- */

#[tauri::command]
pub async fn list_projects(state: State<'_, UiState>) -> Result<Vec<ProjectView>, UiError> {
    project_actions::list(&state).await
}

#[tauri::command]
pub async fn create_project(state: State<'_, UiState>, project: ProjectForm) -> Result<ProjectDataView, UiError> {
    project_actions::create(&state, project).await
}

#[tauri::command]
pub async fn open_project(state: State<'_, UiState>, id: i64) -> Result<ProjectDataView, UiError> {
    project_actions::open(&state, id).await
}

#[tauri::command]
pub async fn update_project(state: State<'_, UiState>, id: i64, project: ProjectForm) -> Result<ProjectDataView, UiError> {
    project_actions::update(&state, id, project).await
}

#[tauri::command]
pub async fn set_project_ready(state: State<'_, UiState>, id: i64, ready: bool) -> Result<ProjectDataView, UiError> {
    project_actions::set_ready(&state, id, ready).await
}

#[tauri::command]
pub async fn delete_project(state: State<'_, UiState>, id: i64) -> Result<(), UiError> {
    project_actions::delete(&state, id).await
}

#[tauri::command]
pub async fn create_scheme(state: State<'_, UiState>, project: i64, scheme: SchemeForm) -> Result<ProjectDataView, UiError> {
    project_actions::create_scheme(&state, project, scheme).await
}

#[tauri::command]
pub async fn update_scheme(state: State<'_, UiState>, id: i64, scheme: SchemeForm) -> Result<ProjectDataView, UiError> {
    project_actions::update_scheme(&state, id, scheme).await
}

#[tauri::command]
pub async fn accept_schemes(state: State<'_, UiState>, project: i64, ids: Vec<i64>) -> Result<ProjectDataView, UiError> {
    project_actions::accept_schemes(&state, project, ids).await
}

#[tauri::command]
pub async fn delete_schemes(state: State<'_, UiState>, project: i64, ids: Vec<i64>) -> Result<ProjectDataView, UiError> {
    project_actions::delete_schemes(&state, project, ids).await
}

/// Reads the table of contents on `range` with AI and stores its
/// chapters as proposed schemes; the progress arrives through
/// `on_event`, "Stop" is `stop_work`.
#[tauri::command]
pub async fn propose_schemes(
    state: State<'_, UiState>,
    project: i64,
    range: PageRange,
    on_event: Channel<AnalysisEvent>,
) -> Result<ProjectDataView, UiError> {
    project_actions::propose_schemes(&state, project, range, move |event| on_event.send(event).is_ok()).await
}

/// Asks where to save the file of a project, and writes it; `false`
/// when the user closed the dialog.
#[tauri::command]
pub async fn export_project(app: AppHandle, state: State<'_, UiState>, id: i64, with_pdfs: bool) -> Result<bool, UiError> {
    let name = project_actions::open(&state, id).await?.project.name;
    let file_name: String = name
        .chars()
        .map(|character| if character.is_alphanumeric() || "-_ .".contains(character) { character } else { '_' })
        .collect();

    let chosen = tokio::task::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Export the test project")
            .set_file_name(format!("{}.project.json", file_name.trim()))
            .add_filter("Test project", &["json"])
            .blocking_save_file()
    })
    .await
    .map_err(|error| UiError::Failed(format!("the dialog failed: {error}")))?;

    let Some(path) = chosen.and_then(|file| file.into_path().ok()) else {
        return Ok(false);
    };

    project_actions::export(&state, id, with_pdfs, &path).await?;

    Ok(true)
}

/// Asks for the file of a project and imports it; `None` when the user
/// closed the dialog. Documents loaded from the file are reported
/// through `on_event`.
#[tauri::command]
pub async fn import_project(
    app: AppHandle,
    state: State<'_, UiState>,
    on_event: Channel<LoadEvent>,
) -> Result<Option<ProjectDataView>, UiError> {
    state.session()?;

    let chosen = tokio::task::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Import a test project")
            .add_filter("Test project", &["json"])
            .blocking_pick_file()
    })
    .await
    .map_err(|error| UiError::Failed(format!("the dialog failed: {error}")))?;

    let Some(path) = chosen.and_then(|file| file.into_path().ok()) else {
        return Ok(None);
    };

    Ok(Some(project_actions::import(&state, &path, move |event| on_event.send(event).is_ok()).await?))
}

/* ---------------------------------------------------------------
 * Create evaluation scheme: the drawing of a scheme
 * ------------------------------------------------------------- */

#[tauri::command]
pub async fn open_scheme(state: State<'_, UiState>, id: i64) -> Result<SchemeDataView, UiError> {
    scheme_actions::open(&state, id).await
}

#[tauri::command]
pub async fn add_tile(state: State<'_, UiState>, scheme: i64, tile: TileForm) -> Result<SchemeDataView, UiError> {
    scheme_actions::add_tile(&state, scheme, tile).await
}

#[tauri::command]
pub async fn update_tile(state: State<'_, UiState>, id: i64, tile: TileForm) -> Result<SchemeDataView, UiError> {
    scheme_actions::update_tile(&state, id, tile).await
}

#[tauri::command]
pub async fn move_tile(state: State<'_, UiState>, id: i64, x: f64, y: f64) -> Result<(), UiError> {
    scheme_actions::move_tile(&state, id, x, y).await
}

/// Moves tiles to the scheme `to`; gives back the scheme `scheme`
/// shown.
#[tauri::command]
pub async fn move_tiles(state: State<'_, UiState>, scheme: i64, ids: Vec<i64>, to: i64) -> Result<SchemeDataView, UiError> {
    scheme_actions::move_tiles(&state, scheme, ids, to).await
}

#[tauri::command]
pub async fn delete_tiles(state: State<'_, UiState>, scheme: i64, ids: Vec<i64>) -> Result<SchemeDataView, UiError> {
    scheme_actions::delete_tiles(&state, scheme, ids).await
}

#[tauri::command]
pub async fn accept_tiles(state: State<'_, UiState>, scheme: i64, ids: Vec<i64>) -> Result<SchemeDataView, UiError> {
    scheme_actions::accept_tiles(&state, scheme, ids).await
}

#[tauri::command]
pub async fn add_link(state: State<'_, UiState>, scheme: i64, from: i64, to: i64) -> Result<SchemeDataView, UiError> {
    scheme_actions::add_link(&state, scheme, from, to).await
}

#[tauri::command]
pub async fn delete_link(state: State<'_, UiState>, scheme: i64, id: i64) -> Result<SchemeDataView, UiError> {
    scheme_actions::delete_link(&state, scheme, id).await
}

#[tauri::command]
pub async fn undo_scheme(state: State<'_, UiState>, scheme: i64) -> Result<SchemeDataView, UiError> {
    scheme_actions::undo(&state, scheme).await
}

/// The instructions `which` ("chapter" or "contents") of the AI.
#[tauri::command]
pub async fn analysis_instructions(state: State<'_, UiState>, which: String) -> Result<String, UiError> {
    scheme_actions::instructions(&state, which).await
}

/// Stores the instructions `which`; empty text: the default.
#[tauri::command]
pub async fn save_analysis_instructions(state: State<'_, UiState>, which: String, text: String) -> Result<String, UiError> {
    scheme_actions::save_instructions(&state, which, text).await
}

/// Analyses pages of the documents of the project for one scheme with
/// AI (no ranges: the pages of the scheme); the progress arrives
/// through `on_event`, "Stop" is `stop_work`.
#[tauri::command]
pub async fn analyse_scheme(
    state: State<'_, UiState>,
    scheme: i64,
    ranges: Vec<PageRange>,
    part_size: usize,
    on_event: Channel<AnalysisEvent>,
) -> Result<SchemeDataView, UiError> {
    scheme_actions::analyse(&state, scheme, ranges, part_size, move |event| on_event.send(event).is_ok()).await
}

/// Analyses several schemes of a project, each on its own pages.
#[tauri::command]
pub async fn analyse_schemes(
    state: State<'_, UiState>,
    project: i64,
    ids: Vec<i64>,
    part_size: usize,
    on_event: Channel<AnalysisEvent>,
) -> Result<ProjectDataView, UiError> {
    scheme_actions::analyse_schemes(&state, project, ids, part_size, move |event| on_event.send(event).is_ok()).await
}

/// The PDF of a document, as raw bytes (an ArrayBuffer in the page),
/// for the viewer.
#[tauri::command]
pub async fn document_pdf(state: State<'_, UiState>, id: i64) -> Result<Response, UiError> {
    Ok(Response::new(standard_actions::pdf(&state, id).await?))
}
