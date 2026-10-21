//! The catalogs of OSCAL in the tool Evaluation plan (step 41.3): made
//! from the documents of a standard, edited group by group and control
//! by control, their quotes, and the proposals of the AI
//! (`business/oscal/catalogs.rs`, `quotes.rs`, `proposals.rs`,
//! `building.rs`). Every operation gives the page the catalog as it is
//! now, so that it never has to guess what the program stored.

use serde_json::Value;

use crate::ai::api::tasks;
use crate::business::oscal::catalogs::{self, Place};
use crate::business::oscal::{building, documents, proposals, quotes};
use crate::business::work::Cancel;
use crate::db::user::database::UserDb;
use crate::model::ai::Task;
use crate::model::oscal::catalog::{Control, Group};

use crate::tauri_ui::error::UiError;
use crate::tauri_ui::serializer::catalog_view::{BuildEvent, CatalogView, PageRange, object};
use crate::tauri_ui::session::with_user_db;
use crate::tauri_ui::state::UiState;
use crate::views::plans::oscal_documents::view::OscalDocumentView;

/// The catalog `uuid` as the editor shows it.
fn view(db: &UserDb, uuid: &str) -> Result<CatalogView, UiError> {
    let summary =
        documents::get(db, uuid)?.ok_or(UiError::Invalid("the OSCAL document does not exist".to_owned()))?.summary();
    let catalog = catalogs::get(db, uuid)?;

    CatalogView::new(
        OscalDocumentView::new(summary),
        &catalog,
        quotes::list(db, uuid, None)?,
        proposals::list(db, uuid)?,
    )
}

pub async fn open(state: &UiState, uuid: String) -> Result<CatalogView, UiError> {
    with_user_db(state, move |db| view(db, &uuid)).await
}

/// Makes one change of catalog `uuid` and gives it back as it is now.
async fn change(
    state: &UiState,
    uuid: String,
    work: impl FnOnce(&UserDb, &str) -> Result<(), UiError> + Send + 'static,
) -> Result<CatalogView, UiError> {
    with_user_db(state, move |db| {
        work(db, &uuid)?;
        view(db, &uuid)
    })
    .await
}

pub async fn add_group(state: &UiState, uuid: String, place: Place, group: Value) -> Result<CatalogView, UiError> {
    let group: Group = object(group)?;

    change(state, uuid, move |db, uuid| Ok(catalogs::add_group(db, uuid, &place, group)?)).await
}

pub async fn update_group(state: &UiState, uuid: String, id: String, group: Value) -> Result<CatalogView, UiError> {
    let group: Group = object(group)?;

    change(state, uuid, move |db, uuid| Ok(catalogs::update_group(db, uuid, &id, group)?)).await
}

pub async fn add_control(state: &UiState, uuid: String, place: Place, control: Value) -> Result<CatalogView, UiError> {
    let control: Control = object(control)?;

    change(state, uuid, move |db, uuid| Ok(catalogs::add_control(db, uuid, &place, control)?)).await
}

pub async fn update_control(state: &UiState, uuid: String, id: String, control: Value) -> Result<CatalogView, UiError> {
    let control: Control = object(control)?;

    change(state, uuid, move |db, uuid| Ok(catalogs::update_control(db, uuid, &id, control)?)).await
}

pub async fn delete_object(state: &UiState, uuid: String, id: String) -> Result<CatalogView, UiError> {
    change(state, uuid, move |db, uuid| Ok(catalogs::delete_object(db, uuid, &id)?)).await
}

/// Adds a quote to an object (looked for on its page).
pub async fn add_quote(
    state: &UiState,
    uuid: String,
    object_id: String,
    source: i64,
    page: u32,
    text: String,
) -> Result<CatalogView, UiError> {
    change(state, uuid, move |db, uuid| {
        quotes::create(db, uuid, &object_id, source, page, &text)?;
        Ok(())
    })
    .await
}

pub async fn delete_quote(state: &UiState, uuid: String, id: i64) -> Result<CatalogView, UiError> {
    change(state, uuid, move |db, _| {
        quotes::delete(db, id)?;
        Ok(())
    })
    .await
}

/// Accepts a proposal of the AI, as it is or as `corrected` by the
/// evaluator (its JSON of OSCAL).
pub async fn accept_proposal(
    state: &UiState,
    uuid: String,
    id: i64,
    corrected: Option<Value>,
) -> Result<CatalogView, UiError> {
    change(state, uuid, move |db, _| {
        proposals::accept(db, id, corrected)?;
        Ok(())
    })
    .await
}

pub async fn reject_proposal(state: &UiState, uuid: String, id: i64) -> Result<CatalogView, UiError> {
    change(state, uuid, move |db, _| {
        proposals::reject(db, id)?;
        Ok(())
    })
    .await
}

/// The task of instructions `which` ("catalog_outline" or
/// "catalog_section").
fn instruction_task(which: &str) -> Result<Task, UiError> {
    Task::from_name(which)
        .filter(|task| tasks::default_instructions(*task).is_some())
        .ok_or_else(|| UiError::Invalid("there are no such instructions".to_owned()))
}

/// The instructions `which`: those the evaluator changed, or the
/// default ones.
pub async fn instructions(state: &UiState, which: String) -> Result<String, UiError> {
    let task = instruction_task(&which)?;

    with_user_db(state, move |db| Ok(tasks::instructions(db, task)?.unwrap_or_default())).await
}

/// Stores the instructions `which`; empty text brings the default ones
/// back. Gives the instructions now in force.
pub async fn save_instructions(state: &UiState, which: String, text: String) -> Result<String, UiError> {
    let task = instruction_task(&which)?;

    with_user_db(state, move |db| Ok(tasks::set_instructions(db, task, &text)?.unwrap_or_default())).await
}

/// The AI reads a table of contents and proposes the groups
/// (`business/oscal/building.rs`).
pub async fn propose_groups(
    state: &UiState,
    uuid: String,
    range: PageRange,
    events: impl Fn(BuildEvent) -> bool + Send + Sync + Clone + 'static,
) -> Result<CatalogView, UiError> {
    let session = state.session()?;
    let cancel = Cancel::new(state.start_work());
    let progress = move |progress: building::Progress| events(progress.into());

    building::propose_groups(
        &session,
        uuid.clone(),
        range.document_id,
        range.first_page,
        range.last_page,
        cancel,
        progress,
    )
    .await?;

    open(state, uuid).await
}

/// The AI reads pages of a group and proposes its controls.
pub async fn propose_controls(
    state: &UiState,
    uuid: String,
    group: Option<String>,
    ranges: Vec<PageRange>,
    part_size: usize,
    events: impl Fn(BuildEvent) -> bool + Send + Sync + Clone + 'static,
) -> Result<CatalogView, UiError> {
    let session = state.session()?;
    let cancel = Cancel::new(state.start_work());
    let progress = move |progress: building::Progress| events(progress.into());
    let ranges = ranges.into_iter().map(|range| (range.document_id, range.first_page, range.last_page)).collect();

    building::propose_controls(&session, uuid.clone(), group, ranges, part_size, cancel, progress).await?;

    open(state, uuid).await
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "catalogs_tests.rs"]
mod tests;
