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

pub mod catalogs;

use tauri::State;
use tauri::ipc::Channel;

use super::error::UiError;
use super::serializer::catalog_view::{BuildEvent, CatalogView, PageRange, PlaceForm};
use super::state::UiState;

/* ---------------------------------------------------------------
 * The editor of a catalog (moved to views/ next, step 42)
 * ------------------------------------------------------------- */

/// Stops the long work running (loading, an analysis).
#[tauri::command]
pub fn stop_work(state: State<'_, UiState>) {
    state.stop_work();
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
