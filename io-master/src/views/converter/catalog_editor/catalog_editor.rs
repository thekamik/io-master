//! The entry of the editor of a catalog, a screen of the view Converter: a
//! catalog of OSCAL made in the program from the documents (PDF) of
//! its standard, group by group and control by control, with the AI (page:
//! `ui/src/views/converter/catalog_editor/`, its commands in
//! `ui/src/lib/api/converter/catalog_editor.ts`). Which object is
//! chosen, and what the viewer shows, are presentation, kept by the page;
//! every element that changes the catalog or its evaluation support tells
//! the editor, which asks every element to read its data again. Nothing the
//! AI proposes enters the catalog until the evaluator accepts it, as it is
//! or corrected (P1 of docs/architecture.md). The flow of data:
//! `docs/architecture.md`, section 12.

use serde_json::Value;
use tauri::State;
use tauri::ipc::Channel;

use crate::ai::api::tasks;
use crate::app::error::AppError;
use crate::app::state::AppState;
use crate::db::conversion as db_conversion;
use crate::db::evaluation_support as db_evaluation_support;
use crate::db::knowledge as db_knowledge;
use crate::db::oscal as db_oscal;
use crate::db::oscal::error::OscalError;
use crate::db::standards as db_standards;
use crate::model_old::ai::Task;
use crate::model_old::conversion::{GroupState, Stage};
use crate::model_old::knowledge::AgentPlace;
use crate::model_old::oscal::catalog::{Control, Group};

use super::ai_builder::view::{self as builder_view, BuildEvent, PageRange, SourceView};
use super::ai_builder::{controls, groups};
use super::conversion_wizard::view::{PagesForm, WizardView};
use super::conversion_wizard::{contents, locate, progress};
use super::new_object::view::PlaceForm;
use super::object_form::oscal_object;
use super::object_form::view::ObjectView;
use super::object_quotes::checked_quote;
use super::object_quotes::view::ObjectQuotesView;
use super::proposals::accept;
use super::proposals::view::{self as proposal_view, ProposalView};

// ── When the view opens ───────────────────────────────────────────────

/// How far the conversion of catalog `uuid` is: its stage, the pages of its
/// table of contents, its groups with their pages and state, and what is
/// left to check (step 50.2 of docs/history.md).
///
/// Called by the page: `ConversionWizard.svelte`, when the editor opens and
/// after every change (`catalogEditor.conversionWizardGet(uuid)`).
/// Calls: `progress::read`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn conversion_wizard_get(state: State<'_, AppState>, uuid: String) -> Result<WizardView, AppError> {
    let session = state.session()?;

    progress::read(&session.db(), &uuid)
}

/// The proposals waiting for catalog `uuid`, the oldest first.
///
/// Called by the page: `Proposals.svelte`, when the editor opens and after
/// every change (`catalogEditor.proposalsGet(uuid)`).
/// Calls: `db_evaluation_support::read::proposals`,
/// `db_standards::read::documents`, `proposal_view::all`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn proposals_get(state: State<'_, AppState>, uuid: String) -> Result<Vec<ProposalView>, AppError> {
    let session = state.session()?;
    let db = session.db();

    proposal_view::all(db_evaluation_support::read::proposals(&db, &uuid)?, &db_standards::read::documents(&db)?)
}

/// The documents the AI can read, by title and part.
///
/// Called by the page: `AiBuilder.svelte`, when the editor opens
/// (`catalogEditor.aiBuilderSources()`).
/// Calls: `db_standards::read::documents`, `builder_view::sources`.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn ai_builder_sources(state: State<'_, AppState>) -> Result<Vec<SourceView>, AppError> {
    let session = state.session()?;

    Ok(builder_view::sources(&db_standards::read::documents(&session.db())?))
}

/// The instructions `which`: those the evaluator changed, or the default
/// ones.
///
/// Called by the page: `AiInstructions.svelte`, when its dialog opens
/// (`catalogEditor.aiInstructionsGet(which)`).
/// Calls: `instructions_task` (below), `tasks::instructions`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn ai_instructions_get(state: State<'_, AppState>, which: String) -> Result<String, AppError> {
    let session = state.session()?;

    Ok(tasks::instructions(&session.db(), instructions_task(&which)?)?.unwrap_or_default())
}

// ── The actions of the elements ───────────────────────────────────────

/// The group or control `id` of catalog `uuid`.
///
/// Called by the page: `ObjectForm.svelte`, when an object is chosen in the
/// tree (`catalogEditor.objectFormGet(uuid, id)`).
/// Calls: `db_oscal::read::catalog`, `ObjectView::new`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn object_form_get(state: State<'_, AppState>, uuid: String, id: String) -> Result<ObjectView, AppError> {
    let session = state.session()?;
    let catalog = db_oscal::read::catalog(&session.db(), &uuid)?;

    let Some(location) = catalog.locate(&id)? else {
        return Err(OscalError::NoSuchObject(id).into());
    };

    ObjectView::new(&catalog, &location, &id)
}

/// Changes the own fields of group `id` of catalog `uuid` (`group`: its
/// JSON of OSCAL, the id may be new).
///
/// Called by the page: `ObjectForm.svelte`, "Save" of a group
/// (`catalogEditor.objectFormSaveGroup(...)`).
/// Calls: `oscal_object::read`, `db_oscal::read::catalog`, the change of the
/// catalog in memory (`model/oscal/catalog/edit.rs`), `db_oscal::update::catalog`
/// (checked against the schema of NIST).
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn object_form_save_group(
    state: State<'_, AppState>,
    uuid: String,
    id: String,
    group: Value,
) -> Result<(), AppError> {
    let session = state.session()?;
    let group: Group = oscal_object::read(group)?;
    let db = session.db();

    let mut catalog = db_oscal::read::catalog(&db, &uuid)?;
    catalog.update_group(&id, group)?;
    db_oscal::update::catalog(&db, catalog)?;

    Ok(())
}

/// Changes the own fields of control `id` of catalog `uuid`.
///
/// Called by the page: `ObjectForm.svelte`, "Save" of a control
/// (`catalogEditor.objectFormSaveControl(...)`).
/// Calls: `oscal_object::read`, `db_oscal::read::catalog`, the change of the
/// catalog in memory, `db_oscal::update::catalog`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn object_form_save_control(
    state: State<'_, AppState>,
    uuid: String,
    id: String,
    control: Value,
) -> Result<(), AppError> {
    let session = state.session()?;
    let control: Control = oscal_object::read(control)?;
    let db = session.db();

    let mut catalog = db_oscal::read::catalog(&db, &uuid)?;
    catalog.update_control(&id, control)?;
    db_oscal::update::catalog(&db, catalog)?;

    Ok(())
}

/// Deletes the group or control `id` of catalog `uuid` with everything it
/// holds, and the quotes of everything deleted.
///
/// Called by the page: `ObjectForm.svelte`, "Delete"
/// (`catalogEditor.objectFormDelete(uuid, id)`).
/// Calls: `db_oscal::read::catalog`, the removal in memory,
/// `db_oscal::update::catalog`, `db_evaluation_support::delete::quotes_of`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn object_form_delete(state: State<'_, AppState>, uuid: String, id: String) -> Result<(), AppError> {
    let session = state.session()?;
    let db = session.db();

    let mut catalog = db_oscal::read::catalog(&db, &uuid)?;
    let gone = catalog.remove(&id)?;

    db_oscal::update::catalog(&db, catalog)?;
    db_evaluation_support::delete::quotes_of(&db, &uuid, &gone)?;

    Ok(())
}

/// Adds `group` (its JSON of OSCAL) at `place` of catalog `uuid`.
///
/// Called by the page: `NewObject.svelte`, "Add" of a group
/// (`catalogEditor.newObjectAddGroup(...)`).
/// Calls: `oscal_object::read`, `db_oscal::read::catalog`, the addition in
/// memory, `db_oscal::update::catalog`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn new_object_add_group(
    state: State<'_, AppState>,
    uuid: String,
    place: PlaceForm,
    group: Value,
) -> Result<(), AppError> {
    let session = state.session()?;
    let place = place.into_place()?;
    let group: Group = oscal_object::read(group)?;
    let db = session.db();

    let mut catalog = db_oscal::read::catalog(&db, &uuid)?;
    catalog.add_group(&place, group)?;
    db_oscal::update::catalog(&db, catalog)?;

    Ok(())
}

/// Adds `control` (its JSON of OSCAL) at `place` of catalog `uuid`.
///
/// Called by the page: `NewObject.svelte`, "Add" of a control
/// (`catalogEditor.newObjectAddControl(...)`).
/// Calls: `oscal_object::read`, `db_oscal::read::catalog`, the addition in
/// memory, `db_oscal::update::catalog`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn new_object_add_control(
    state: State<'_, AppState>,
    uuid: String,
    place: PlaceForm,
    control: Value,
) -> Result<(), AppError> {
    let session = state.session()?;
    let place = place.into_place()?;
    let control: Control = oscal_object::read(control)?;
    let db = session.db();

    let mut catalog = db_oscal::read::catalog(&db, &uuid)?;
    catalog.add_control(&place, control)?;
    db_oscal::update::catalog(&db, catalog)?;

    Ok(())
}

/// The quotes of object `object_id` of catalog `uuid`.
///
/// Called by the page: `ObjectQuotes.svelte`, when an object is chosen and
/// after a change (`catalogEditor.objectQuotesGet(uuid, objectId)`).
/// Calls: `db_evaluation_support::read::quotes`,
/// `db_standards::read::documents`, `ObjectQuotesView::new`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn object_quotes_get(
    state: State<'_, AppState>,
    uuid: String,
    object_id: String,
) -> Result<ObjectQuotesView, AppError> {
    let session = state.session()?;
    let db = session.db();

    let quotes = db_evaluation_support::read::quotes(&db, &uuid, Some(&object_id))?;

    Ok(ObjectQuotesView::new(quotes, &db_standards::read::documents(&db)?))
}

/// Adds the quote `text` of page `page` of document `source` to object
/// `object_id` of catalog `uuid`, looked for on its page.
///
/// Called by the page: `ObjectQuotes.svelte`, a quote taken in the viewer
/// (`catalogEditor.objectQuotesAdd(...)`).
/// Calls: `db_oscal::read::catalog`, `checked_quote::checked` (the quote
/// looked for on its page), `db_evaluation_support::create::quote`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn object_quotes_add(
    state: State<'_, AppState>,
    uuid: String,
    object_id: String,
    source: i64,
    page: u32,
    text: String,
) -> Result<(), AppError> {
    let session = state.session()?;
    let db = session.db();

    let catalog = db_oscal::read::catalog(&db, &uuid)?;
    ensure!(catalog.ids()?.contains(&object_id), OscalError::NoSuchObject(object_id).into());

    let draft = checked_quote::checked(&db, source, page, &text)?;
    db_evaluation_support::create::quote(&db, &uuid, &object_id, &draft)?;

    Ok(())
}

/// Deletes quote `id`.
///
/// Called by the page: `ObjectQuotes.svelte`, "Delete"
/// (`catalogEditor.objectQuotesDelete(id)`).
/// Calls: `db_evaluation_support::delete::quote`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn object_quotes_delete(state: State<'_, AppState>, id: i64) -> Result<(), AppError> {
    let session = state.session()?;

    ensure!(
        db_evaluation_support::delete::quote(&session.db(), id)?,
        AppError::Invalid("the quote does not exist".to_owned())
    );

    Ok(())
}

/// Accepts proposal `id`, as it is (`corrected` null) or as corrected.
///
/// Called by the page: `Proposals.svelte`, "Accept", as proposed or corrected
/// (`catalogEditor.proposalsAccept(...)`).
/// Calls: `accept::accept` (the object added, its quotes kept, the proposal
/// removed, in one transaction).
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn proposals_accept(state: State<'_, AppState>, id: i64, corrected: Option<Value>) -> Result<(), AppError> {
    let session = state.session()?;

    accept::accept(&session.db(), id, corrected)?;

    Ok(())
}

/// Rejects proposal `id`: it is removed, the catalog is not touched.
///
/// Called by the page: `Proposals.svelte`, "Reject"
/// (`catalogEditor.proposalsReject(id)`).
/// Calls: `db_evaluation_support::delete::proposal`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn proposals_reject(state: State<'_, AppState>, id: i64) -> Result<(), AppError> {
    let session = state.session()?;

    ensure!(
        db_evaluation_support::delete::proposal(&session.db(), id)?,
        AppError::Invalid("the proposal does not exist".to_owned())
    );

    Ok(())
}

/// The AI reads the table of contents in `range` and proposes the groups
/// of catalog `uuid` (`groups.rs`); the events arrive through `on_event`.
///
/// Called by the page: `AiBuilder.svelte`, "Propose the groups"
/// (`catalogEditor.aiBuilderGroups(...)`).
/// Calls: `AppState::start_work` (the stop signal), `groups::propose` (the
/// AI reads a table of contents part by part, through `ai/api/`).
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn ai_builder_groups(
    state: State<'_, AppState>,
    uuid: String,
    range: PageRange,
    on_event: Channel<BuildEvent>,
) -> Result<(), AppError> {
    let session = state.session()?;
    let stop = state.start_work();

    groups::propose(&session, &uuid, &range, &stop, &on_event).await
}

/// The AI reads the pages `ranges` of group `group` (`None`: the top) and
/// proposes its controls (`controls.rs`), at most `part_size` characters
/// per question; the events arrive through `on_event`.
///
/// Called by the page: `AiBuilder.svelte`, "Propose the controls"
/// (`catalogEditor.aiBuilderControls(...)`).
/// Calls: `AppState::start_work`, `controls::propose` (the AI reads the pages
/// of a group part by part, through `ai/api/`).
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn ai_builder_controls(
    state: State<'_, AppState>,
    uuid: String,
    group: Option<String>,
    ranges: Vec<PageRange>,
    part_size: usize,
    on_event: Channel<BuildEvent>,
) -> Result<(), AppError> {
    let session = state.session()?;
    let stop = state.start_work();

    controls::propose(&session, &uuid, group.as_deref(), &ranges, part_size, &stop, &on_event).await
}

/// The AI reads the first pages of document `document_id` and names the
/// pages of its table of contents, kept for the conversion of `uuid` for the
/// evaluator to confirm; the events arrive through `on_event`.
///
/// Called by the page: `ConversionWizard.svelte`, stage "Table of contents",
/// "Find it with the AI" (`catalogEditor.conversionWizardFindContents(...)`).
/// Calls: `AppState::start_work`, `contents::find` (one question through
/// `ai/api/`), `progress::read`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn conversion_wizard_find_contents(
    state: State<'_, AppState>,
    uuid: String,
    document_id: i64,
    on_event: Channel<BuildEvent>,
) -> Result<WizardView, AppError> {
    let session = state.session()?;
    let stop = state.start_work();

    contents::find(&session, &uuid, document_id, &stop, &on_event).await?;

    progress::read(&session.db(), &uuid)
}

/// Sets the pages of the table of contents of the conversion of `uuid`, as
/// the evaluator confirmed or corrected them.
///
/// Called by the page: `ConversionWizard.svelte`, stage "Table of contents",
/// "Keep these pages" (`catalogEditor.conversionWizardContents(...)`).
/// Calls: `progress::checked_pages`, `db_conversion::update::contents`,
/// `progress::read`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn conversion_wizard_contents(
    state: State<'_, AppState>,
    uuid: String,
    pages: PagesForm,
) -> Result<WizardView, AppError> {
    let session = state.session()?;
    let db = session.db();
    let pages = progress::checked_pages(&db, &pages)?;

    db_conversion::update::contents(&db, &uuid, pages)?;

    progress::read(&db, &uuid)
}

/// Moves the conversion of `uuid` to `stage`. Entering the controls, the
/// program finds where every group starts after the table of contents
/// (`locate.rs`), keeping what the evaluator gave a group before; a
/// conversion is finished only from its review.
///
/// Called by the page: `ConversionWizard.svelte`, "Next" and "Back" of every
/// stage, "Finish" (`catalogEditor.conversionWizardStage(uuid, stage)`).
/// Calls: `db_conversion::read`, `locate::locate` and `merge`,
/// `db_conversion::update::groups` and `stage`, `progress::read`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn conversion_wizard_stage(
    state: State<'_, AppState>,
    uuid: String,
    stage: String,
) -> Result<WizardView, AppError> {
    let session = state.session()?;
    let db = session.db();

    let Some(stage) = Stage::from_name(&stage) else {
        return Err(AppError::Invalid("there is no such stage".to_owned()));
    };

    let Some(current) = db_conversion::read::conversion(&db, &uuid)? else {
        return Err(AppError::Invalid("the catalog is not in conversion".to_owned()));
    };

    ensure!(
        stage != Stage::Finished || current.stage == Stage::Review,
        AppError::Invalid("a conversion is finished from its review".to_owned())
    );

    if stage == Stage::Controls {
        let Some(pages) = current.contents else {
            return Err(AppError::Invalid("first the pages of the table of contents".to_owned()));
        };

        let catalog = db_oscal::read::catalog(&db, &uuid)?;
        let found = locate::locate(&db, &catalog, pages)?;
        let known = db_conversion::read::groups(&db, &uuid)?;

        db_conversion::update::groups(&db, &uuid, &locate::merge(found, &known))?;
    }

    db_conversion::update::stage(&db, &uuid, stage)?;

    progress::read(&db, &uuid)
}

/// Sets the pages of group `group` of the conversion of `uuid`, as the
/// evaluator corrected them.
///
/// Called by the page: `ConversionWizard.svelte`, stage "Controls", the pages
/// of a group (`catalogEditor.conversionWizardGroupPages(...)`).
/// Calls: `progress::checked_pages`, `db_conversion::update::group_pages`,
/// `progress::read`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn conversion_wizard_group_pages(
    state: State<'_, AppState>,
    uuid: String,
    group: String,
    pages: PagesForm,
) -> Result<WizardView, AppError> {
    let session = state.session()?;
    let db = session.db();
    let pages = progress::checked_pages(&db, &pages)?;

    db_conversion::update::group_pages(&db, &uuid, &group, pages)?;

    progress::read(&db, &uuid)
}

/// Sets the state of group `group` of the conversion of `uuid`: proposed
/// (the AI read its pages) or done (the evaluator says so).
///
/// Called by the page: `ConversionWizard.svelte`, stage "Controls", after the
/// AI read a group and "This group is done"
/// (`catalogEditor.conversionWizardGroupState(...)`).
/// Calls: `GroupState::from_name`, `db_conversion::update::group_state`,
/// `progress::read`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn conversion_wizard_group_state(
    state: State<'_, AppState>,
    uuid: String,
    group: String,
    group_state: String,
) -> Result<WizardView, AppError> {
    let session = state.session()?;
    let db = session.db();

    let Some(group_state) = GroupState::from_name(&group_state) else {
        return Err(AppError::Invalid("there is no such state of a group".to_owned()));
    };

    db_conversion::update::group_state(&db, &uuid, &group, group_state)?;

    progress::read(&db, &uuid)
}

/// Chooses the agent with knowledge of catalog `uuid` (none: `None`): the
/// proposals of its groups and controls get the passages it finds (step 57.6).
///
/// Called by the page: `ConversionWizard.svelte`, the choice of the agent
/// (`catalogEditor.conversionWizardAgent(uuid, agent)`).
/// Calls: `db_knowledge::update::attachment`, `progress::read`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn conversion_wizard_agent(
    state: State<'_, AppState>,
    uuid: String,
    agent: Option<i64>,
) -> Result<WizardView, AppError> {
    let session = state.session()?;
    let db = session.db();

    progress::read(&db, &uuid)?;
    db_knowledge::update::attachment(&db, AgentPlace::Catalog, &uuid, agent)
        .map_err(|_| AppError::Invalid("the agent does not exist".to_owned()))?;

    progress::read(&db, &uuid)
}

/// Stops the work of the AI, if one runs.
///
/// Called by the page: `AiBuilder.svelte`, "Stop" (`catalogEditor.aiBuilderStop()`).
/// Calls: `AppState::stop_work`.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn ai_builder_stop(state: State<'_, AppState>) {
    state.stop_work();
}

/// Stores the instructions `which`; an empty text brings the default ones
/// back. Gives the instructions now in force.
///
/// Called by the page: `AiInstructions.svelte`, "Save"
/// (`catalogEditor.aiInstructionsSave(which, text)`).
/// Calls: `instructions_task` (below), `tasks::set_instructions`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn ai_instructions_save(state: State<'_, AppState>, which: String, text: String) -> Result<String, AppError> {
    let session = state.session()?;

    Ok(tasks::set_instructions(&session.db(), instructions_task(&which)?, &text)?.unwrap_or_default())
}

// ── Helpers of the commands above ─────────────────────────────────────

/// The task of the instructions `which`, one of those that build a
/// catalog.
///
/// Called by: `ai_instructions_get`, `ai_instructions_save`.
/// Calls: `Task::from_name`.
fn instructions_task(which: &str) -> Result<Task, AppError> {
    let Some(task) = Task::from_name(which) else {
        return Err(AppError::Invalid("there are no such instructions".to_owned()));
    };

    ensure!(
        matches!(task, Task::CatalogContents | Task::CatalogOutline | Task::CatalogSection),
        AppError::Invalid("there are no such instructions".to_owned())
    );

    Ok(task)
}

#[cfg(test)]
mod tests {
    //! Tests of the commands of the editor of a catalog, called as the page
    //! calls them: by name, through the IPC of Tauri. The work of the AI is
    //! tested in `ai_builder/groups.rs` and `ai_builder/controls.rs`: its
    //! events go through a channel, which the mock window does not carry.

    use serde_json::json;

    use crate::db::evaluation_support as db_evaluation_support;
    use crate::db::evaluation_support::create::NewProposal;
    use crate::model_old::evaluation_support::{ProposalKind, QuoteDraft};
    use crate::views::converter::catalog_editor::test_support::{add_control, add_group, with_catalog};
    use crate::views::test_support::{TestWindow, logged_in_window};

    #[test]
    fn the_tree_is_rows_with_their_level() {
        let (window, uuid, _) = with_catalog();
        add_group(&window, &uuid, "ADV");
        add_control(&window, &uuid, "ADV", "ADV_FSP.4").unwrap();

        let tree = window.call("catalog_tree_get", json!({ "uuid": uuid })).unwrap();

        assert_eq!(tree["title"], "CC Part 3");
        assert_eq!(tree["version"], "CC:2022");
        assert_eq!(
            tree["rows"],
            json!([
                { "kind": "group", "id": "ADV", "title": "Group ADV", "level": 0 },
                { "kind": "control", "id": "ADV_FSP.4", "title": "Control ADV_FSP.4", "level": 1 }
            ])
        );
    }

    #[test]
    fn a_document_that_is_not_a_catalog_is_refused() {
        let (window, _, _) = with_catalog();

        assert_eq!(window.call("catalog_tree_get", json!({ "uuid": "no-such" })).unwrap_err()["kind"], "invalid");
    }

    /// Stores a proposal of control `id` for group ADV of catalog `uuid`.
    fn propose(window: &TestWindow, uuid: &str, source: i64, id: &str) -> i64 {
        let content = json!({ "id": id, "title": "Complete", "parts": [{ "name": "statement", "prose": "Shall." }] });
        let quotes = [QuoteDraft { source_document_id: source, page: 2, text: "provide".to_owned(), found: true }];
        let proposal = NewProposal {
            document_uuid: uuid,
            kind: ProposalKind::Control,
            parent_id: Some("ADV"),
            content: &content,
            quotes: &quotes,
            note: "From page 2.",
            model: "fake",
        };

        db_evaluation_support::create::proposal(&window.session().db(), &proposal).unwrap()
    }

    #[test]
    fn proposals_are_shown_accepted_corrected_and_rejected() {
        let (window, uuid, source) = with_catalog();
        add_group(&window, &uuid, "ADV");
        let first = propose(&window, &uuid, source, "ADV_FSP.4");
        let second = propose(&window, &uuid, source, "ADV_FSP.5");

        let waiting = window.call("proposals_get", json!({ "uuid": uuid })).unwrap();
        assert_eq!(waiting[0]["object_id"], "ADV_FSP.4");
        assert_eq!(waiting[0]["parts"][0]["prose"], "Shall.");
        assert_eq!(waiting[0]["quotes"][0]["document"], "Common Criteria — Part 3");

        let corrected = json!({ "id": "ADV_FSP.4", "title": "Complete (corrected)" });
        window.call("proposals_accept", json!({ "id": first, "corrected": corrected })).unwrap();
        window.call("proposals_reject", json!({ "id": second })).unwrap();

        assert_eq!(window.call("proposals_get", json!({ "uuid": uuid })).unwrap(), json!([]));
        let tree = window.call("catalog_tree_get", json!({ "uuid": uuid })).unwrap();
        assert_eq!(tree["rows"][1]["title"], "Complete (corrected)");
        assert_eq!(window.call("proposals_reject", json!({ "id": second })).unwrap_err()["kind"], "invalid");
    }

    #[test]
    fn the_sources_are_the_documents_and_stop_without_a_work_does_nothing() {
        let (window, _, source) = with_catalog();

        let sources = window.call("ai_builder_sources", json!({})).unwrap();

        assert_eq!(sources, json!([{ "id": source, "title": "Common Criteria — Part 3", "page_count": 3 }]));
        window.call("ai_builder_stop", json!({})).unwrap();
    }

    #[test]
    fn the_instructions_are_changed_and_the_default_comes_back() {
        let window = logged_in_window();
        let default = crate::ai::tasks::catalog_outline::DEFAULT_INSTRUCTIONS;

        assert_eq!(window.call("ai_instructions_get", json!({ "which": "catalog_outline" })).unwrap(), default);

        let saved = window.call("ai_instructions_save", json!({ "which": "catalog_outline", "text": "Read it all." }));
        assert_eq!(saved.unwrap(), "Read it all.");

        let back = window.call("ai_instructions_save", json!({ "which": "catalog_outline", "text": "" })).unwrap();
        assert_eq!(back, default);

        assert_eq!(window.call("ai_instructions_get", json!({ "which": "chat" })).unwrap_err()["kind"], "invalid");
    }

    #[test]
    fn an_object_is_read_with_its_parts_and_own_fields() {
        let (window, uuid, _) = with_catalog();
        add_group(&window, &uuid, "ADV");
        add_control(&window, &uuid, "ADV", "ADV_FSP.4").unwrap();

        let group = window.call("object_form_get", json!({ "uuid": uuid, "id": "ADV" })).unwrap();
        assert_eq!(group["kind"], "group");
        // The own fields only: what the group holds is not among them.
        assert!(group["fields"].get("controls").is_none());

        let control = window.call("object_form_get", json!({ "uuid": uuid, "id": "ADV_FSP.4" })).unwrap();
        assert_eq!(control["parts"][0]["prose"], "The developer shall provide.");
        assert_eq!(control["parts"][0]["level"], 0);
    }

    #[test]
    fn a_change_keeps_what_the_object_holds_and_a_deletion_takes_it() {
        let (window, uuid, _) = with_catalog();
        add_group(&window, &uuid, "ADV");
        add_control(&window, &uuid, "ADV", "ADV_FSP.4").unwrap();

        let renamed = json!({ "id": "ADV", "title": "Development" });
        window.call("object_form_save_group", json!({ "uuid": uuid, "id": "ADV", "group": renamed })).unwrap();
        let changed = json!({ "id": "ADV_FSP.4", "title": "Complete" });
        window
            .call("object_form_save_control", json!({ "uuid": uuid, "id": "ADV_FSP.4", "control": changed }))
            .unwrap();

        let tree = window.call("catalog_tree_get", json!({ "uuid": uuid })).unwrap();
        assert_eq!(tree["rows"][0]["title"], "Development");
        assert_eq!(tree["rows"][1]["title"], "Complete");

        window.call("object_form_delete", json!({ "uuid": uuid, "id": "ADV" })).unwrap();
        assert_eq!(window.call("catalog_tree_get", json!({ "uuid": uuid })).unwrap()["rows"], json!([]));
        assert_eq!(
            window.call("object_form_delete", json!({ "uuid": uuid, "id": "ADV" })).unwrap_err()["kind"],
            "invalid"
        );
    }

    #[test]
    fn what_is_not_oscal_is_refused() {
        let (window, uuid, _) = with_catalog();
        add_group(&window, &uuid, "ADV");

        let error =
            window.call("object_form_save_group", json!({ "uuid": uuid, "id": "ADV", "group": { "title": 5 } }));

        assert_eq!(error.unwrap_err()["kind"], "invalid");
    }

    #[test]
    fn an_id_is_used_once_and_a_place_must_exist() {
        let (window, uuid, _) = with_catalog();
        add_group(&window, &uuid, "ADV");
        add_control(&window, &uuid, "ADV", "ADV_FSP.4").unwrap();

        assert_eq!(add_control(&window, &uuid, "ADV", "ADV_FSP.4").unwrap_err()["kind"], "invalid");
        assert_eq!(add_control(&window, &uuid, "ATE", "ATE_IND.2").unwrap_err()["kind"], "invalid");

        let place = json!({ "kind": "elsewhere" });
        let group = json!({ "id": "X", "title": "X" });
        let error = window.call("new_object_add_group", json!({ "uuid": uuid, "place": place, "group": group }));
        assert_eq!(error.unwrap_err()["kind"], "invalid");
    }

    #[test]
    fn an_enhancement_goes_into_its_control() {
        let (window, uuid, _) = with_catalog();
        add_group(&window, &uuid, "ADV");
        add_control(&window, &uuid, "ADV", "ADV_FSP.4").unwrap();

        let enhancement = json!({ "id": "ADV_FSP.4.a", "title": "More" });
        let place = json!({ "kind": "control", "id": "ADV_FSP.4" });
        window.call("new_object_add_control", json!({ "uuid": uuid, "place": place, "control": enhancement })).unwrap();

        let tree = window.call("catalog_tree_get", json!({ "uuid": uuid })).unwrap();
        assert_eq!(tree["rows"][2]["id"], "ADV_FSP.4.a");
        assert_eq!(tree["rows"][2]["level"], 2);
    }

    #[test]
    fn quotes_are_looked_for_on_their_pages_and_go_with_their_object() {
        let (window, uuid, source) = with_catalog();
        add_group(&window, &uuid, "ADV");
        add_control(&window, &uuid, "ADV", "ADV_FSP.4").unwrap();
        let add = |text: &str| {
            window.call(
                "object_quotes_add",
                json!({ "uuid": uuid, "objectId": "ADV_FSP.4", "source": source, "page": 2, "text": text }),
            )
        };

        add("The developer shall provide a functional specification.").unwrap();
        add("The evaluator shall dance.").unwrap();
        assert_eq!(add("").unwrap_err()["kind"], "invalid");

        let quotes = window.call("object_quotes_get", json!({ "uuid": uuid, "objectId": "ADV_FSP.4" })).unwrap();
        assert_eq!(quotes["quotes"][0]["found"], true, "across the divided word");
        assert_eq!(quotes["quotes"][1]["found"], false, "made up");
        assert_eq!(quotes["quotes"][0]["document"], "Common Criteria — Part 3");
        assert_eq!(quotes["first_source"], source);

        // A document of a standard that a quote names stays.
        assert_eq!(window.call("standard_documents_delete", json!({ "id": source })).unwrap_err()["kind"], "invalid");

        let id = quotes["quotes"][1]["id"].clone();
        window.call("object_quotes_delete", json!({ "id": id })).unwrap();

        window.call("object_form_delete", json!({ "uuid": uuid, "id": "ADV" })).unwrap();
        let after = window.call("object_quotes_get", json!({ "uuid": uuid, "objectId": "ADV_FSP.4" })).unwrap();
        assert_eq!(after["quotes"], json!([]));
    }

    #[test]
    fn a_quote_of_no_object_is_refused() {
        let (window, uuid, source) = with_catalog();

        let error = window
            .call(
                "object_quotes_add",
                json!({ "uuid": uuid, "objectId": "NONE", "source": source, "page": 1, "text": "Part 3" }),
            )
            .unwrap_err();

        assert_eq!(error["kind"], "invalid");
    }

    /// Adds a group `id` titled `title` at the top, as the dialog of a new
    /// object does.
    fn add_titled_group(window: &TestWindow, uuid: &str, id: &str, title: &str) {
        let group = json!({ "id": id, "title": title });

        window
            .call("new_object_add_group", json!({ "uuid": uuid, "place": { "kind": "top" }, "group": group }))
            .unwrap();
    }

    #[test]
    fn a_new_catalog_starts_its_conversion_at_the_table_of_contents() {
        let (window, uuid, _) = with_catalog();

        let wizard = window.call("conversion_wizard_get", json!({ "uuid": uuid })).unwrap();

        assert_eq!(wizard["stage"], "contents");
        assert_eq!(wizard["contents"], json!(null));
        assert_eq!(wizard["groups"], json!([]));

        // Not finished: the view Evaluation plan does not show it yet.
        assert_eq!(window.call("oscal_documents_get", json!({})).unwrap(), json!([]));
    }

    #[test]
    fn the_wizard_goes_from_the_contents_to_the_controls_of_every_group() {
        let (window, uuid, source) = with_catalog();
        add_titled_group(&window, &uuid, "adv_fsp", "Complete functional specification");
        add_titled_group(&window, &uuid, "ate_ind", "Independent testing");
        let pages = |first: u32, last: u32| json!({ "document_id": source, "first_page": first, "last_page": last });

        // The controls need the table of contents first.
        let early = window.call("conversion_wizard_stage", json!({ "uuid": uuid, "stage": "controls" })).unwrap_err();
        assert_eq!(early["kind"], "invalid");

        window.call("conversion_wizard_contents", json!({ "uuid": uuid, "pages": pages(1, 1) })).unwrap();
        let wizard = window.call("conversion_wizard_stage", json!({ "uuid": uuid, "stage": "controls" })).unwrap();

        // Every group starts where its heading is found.
        assert_eq!(wizard["stage"], "controls");
        assert_eq!(wizard["groups"][0]["id"], "adv_fsp");
        assert_eq!(wizard["groups"][0]["pages"], pages(2, 2));
        assert_eq!(wizard["groups"][1]["pages"], pages(3, 3));
        assert_eq!(wizard["groups"][0]["state"], "todo");

        let call = json!({ "uuid": uuid, "group": "ate_ind", "pages": pages(2, 3) });
        window.call("conversion_wizard_group_pages", call).unwrap();
        let call = json!({ "uuid": uuid, "group": "adv_fsp", "groupState": "done" });
        let wizard = window.call("conversion_wizard_group_state", call).unwrap();

        assert_eq!(wizard["groups"][0]["state"], "done");
        assert_eq!(wizard["groups"][1]["pages"], pages(2, 3));
        assert_eq!(wizard["review"]["groups_without_controls"], 2);

        // Back to the groups and forward again: what the evaluator gave stays.
        window.call("conversion_wizard_stage", json!({ "uuid": uuid, "stage": "groups" })).unwrap();
        let wizard = window.call("conversion_wizard_stage", json!({ "uuid": uuid, "stage": "controls" })).unwrap();
        assert_eq!(wizard["groups"][0]["state"], "done");
        assert_eq!(wizard["groups"][1]["pages"], pages(2, 3));
    }

    #[test]
    fn pages_outside_the_document_are_refused() {
        let (window, uuid, source) = with_catalog();
        let pages = json!({ "document_id": source, "first_page": 2, "last_page": 99 });

        let error = window.call("conversion_wizard_contents", json!({ "uuid": uuid, "pages": pages })).unwrap_err();

        assert_eq!(error["kind"], "invalid");
    }

    #[test]
    fn a_conversion_is_finished_from_its_review_and_then_shown_in_the_plan() {
        let (window, uuid, _) = with_catalog();

        let early = window.call("conversion_wizard_stage", json!({ "uuid": uuid, "stage": "finished" })).unwrap_err();
        assert_eq!(early["kind"], "invalid");

        window.call("conversion_wizard_stage", json!({ "uuid": uuid, "stage": "review" })).unwrap();
        let wizard = window.call("conversion_wizard_stage", json!({ "uuid": uuid, "stage": "finished" })).unwrap();

        assert_eq!(wizard["stage"], "finished");
        assert_eq!(window.call("oscal_documents_get", json!({})).unwrap()[0]["uuid"], uuid);
    }

    #[test]
    fn an_imported_catalog_is_finished_and_not_in_conversion() {
        let window = logged_in_window();
        let uuid = {
            let session = window.session();
            let catalog = crate::test_support::db::basic_catalog();
            crate::db::oscal::create::document(&session.db(), &catalog).unwrap();
            catalog.uuid().to_owned()
        };

        assert_eq!(window.call("conversion_wizard_get", json!({ "uuid": uuid })).unwrap()["stage"], "finished");
        let error = window.call("conversion_wizard_stage", json!({ "uuid": uuid, "stage": "groups" })).unwrap_err();
        assert_eq!(error["kind"], "invalid");
    }

    #[test]
    fn a_catalog_keeps_the_agent_with_knowledge_of_its_conversion() {
        let (window, uuid, _) = with_catalog();
        let agent = crate::test_support::db::knowledge_agent(&window.session().db(), "Roles.");

        let wizard = window.call("conversion_wizard_agent", json!({ "uuid": uuid, "agent": agent })).unwrap();
        assert_eq!(wizard["agent"], agent);

        let wrong = window.call("conversion_wizard_agent", json!({ "uuid": uuid, "agent": 99 })).unwrap_err();
        assert_eq!(wrong["kind"], "invalid");
        let none = window.call("conversion_wizard_agent", json!({ "uuid": uuid, "agent": null })).unwrap();
        assert_eq!(none["agent"], json!(null));
    }
}
