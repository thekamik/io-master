//! The entry of the view Knowledge (RAG) (step 57 of docs/todo.md; page:
//! `ui/src/views/knowledge/`, its commands in `ui/src/lib/api/knowledge.ts`):
//! the knowledge bases of the AI. The evaluator makes a base for a standard
//! or a task, gives it sources (a PDF, a text file, a catalog of OSCAL, a
//! note), which are cut into passages and indexed, and searches it; makes
//! agents of the bases (instructions, glossary, limits, the bases joined in
//! its form), tries them on the proving ground (57.3), and exports and
//! imports an agent with its bases as one file (57.7). The works of the
//! AI of the other views only choose an agent (57.4, 57.6). The flow of
//! data: `docs/architecture.md`, section 12.

use tauri::ipc::Channel;
use tauri::{AppHandle, Runtime, State};

use crate::app::error::AppError;
use crate::app::state::AppState;
use crate::components::ai_work::view::AiEvent;
use crate::components::knowledge_agent::find;
use crate::db_old::knowledge as db_knowledge;
use crate::db_old::user::database::UserDb;
use crate::model_old::knowledge::{check_agent, check_base};
use crate::model_old::text::safe_file_name;

use super::agent_form::dialog::choose_place;
use super::agent_form::export as agent_export;
use super::agent_form::view::{self as agent_view, AgentForm, AgentFormView};
use super::agent_list::dialog::choose_file;
use super::agent_list::import as agent_import;
use super::agent_list::view::{self as agents_view, AgentSummaryView};

use super::knowledge_base::add as source_add;
use super::knowledge_base::dialog::choose_source;
use super::knowledge_base::view::{self as base_view, BaseScreenView, CatalogChoiceView};
use super::knowledge_list::view::{self as list_view, BaseForm, BaseSummaryView};
use super::knowledge_search::search as base_search;
use super::knowledge_search::view::{self as search_view, HitView};
use super::proving_ground::ask as trial_ask;
use super::proving_ground::view::{self as trial_view, TrialView};

// ── The bases ─────────────────────────────────────────────────────────

/// The knowledge bases, by name, with the numbers of their sources and passages.
///
/// Called by the page: `KnowledgeList.svelte`, when the view opens and after a
/// change (`knowledge.knowledgeListGet()`).
/// Calls: `db_knowledge::read::bases`, `list_view::all`.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn knowledge_list_get(state: State<'_, AppState>) -> Result<Vec<BaseSummaryView>, AppError> {
    let session = state.session()?;

    Ok(list_view::all(db_knowledge::read::bases(&session.db())?))
}

/// Makes a new knowledge base of `form`; gives its screen.
///
/// Called by the page: `KnowledgeList.svelte`, "Make" (`knowledge.knowledgeListCreate(form)`).
/// Calls: `check_base`, `db_knowledge::create::base`, `base_view::screen`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn knowledge_list_create(state: State<'_, AppState>, form: BaseForm) -> Result<BaseScreenView, AppError> {
    let session = state.session()?;
    let db = session.db();
    let name = form.name.trim();

    check_base(name, &form.description).map_err(|error| AppError::Invalid(error.to_string()))?;
    let id = db_knowledge::create::base(&db, name, &form.description)?;

    base_view::screen(&db, id)
}

/// Deletes knowledge base `id` with its sources and passages.
///
/// Called by the page: `KnowledgeList.svelte`, "Delete" (`knowledge.knowledgeListDelete(id)`).
/// Calls: `db_knowledge::delete::base`, `list_view::all`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn knowledge_list_delete(state: State<'_, AppState>, id: i64) -> Result<Vec<BaseSummaryView>, AppError> {
    let session = state.session()?;
    let db = session.db();

    ensure!(db_knowledge::delete::base(&db, id)?, AppError::Invalid("the knowledge base does not exist".to_owned()));

    Ok(list_view::all(db_knowledge::read::bases(&db)?))
}

// ── A base ────────────────────────────────────────────────────────────

/// The screen of knowledge base `id`: its name, description and sources.
///
/// Called by the page: `KnowledgeBase.svelte`, when a base opens (`knowledge.knowledgeBaseGet(id)`).
/// Calls: `base_view::screen`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn knowledge_base_get(state: State<'_, AppState>, id: i64) -> Result<BaseScreenView, AppError> {
    let session = state.session()?;

    base_view::screen(&session.db(), id)
}

/// Gives knowledge base `id` the name and the description of `form`.
///
/// Called by the page: `KnowledgeBase.svelte`, "Save" (`knowledge.knowledgeBaseSave(id, form)`).
/// Calls: `check_base`, `db_knowledge::update::base`, `base_view::screen`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn knowledge_base_save(state: State<'_, AppState>, id: i64, form: BaseForm) -> Result<BaseScreenView, AppError> {
    let session = state.session()?;
    let db = session.db();
    let name = form.name.trim();

    check_base(name, &form.description).map_err(|error| AppError::Invalid(error.to_string()))?;
    ensure!(
        db_knowledge::update::base(&db, id, name, &form.description)?,
        AppError::Invalid("the knowledge base does not exist".to_owned())
    );

    base_view::screen(&db, id)
}

/// Asks for a PDF or a text file in a dialog of the system and gives it to
/// knowledge base `id`, cut into passages; `None` when no file was chosen.
///
/// Called by the page: `KnowledgeBase.svelte`, "Add a file…" (`knowledge.knowledgeBaseAddFile(id)`).
/// Calls: [`existing`], `choose_source`, `source_add::read_file` (on a
/// thread for blocking work: Poppler reads a PDF), `source_add::add_file`, `base_view::screen`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn knowledge_base_add_file<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    id: i64,
) -> Result<Option<BaseScreenView>, AppError> {
    let session = state.session()?;
    existing(&session.db(), id)?;

    let Some(path) = choose_source(app).await? else {
        return Ok(None);
    };

    // spawn_blocking takes a closure: a library place (rule 9 of CLAUDE.md).
    let read = tokio::task::spawn_blocking(move || source_add::read_file(&path))
        .await
        .map_err(|_| AppError::Failed("the file was not read".to_owned()))??;
    let db = session.db();
    source_add::add_file(&db, id, &read)?;

    Ok(Some(base_view::screen(&db, id)?))
}

/// Gives knowledge base `id` the catalog of OSCAL `catalog` of the installation.
///
/// Called by the page: `KnowledgeBase.svelte`, "Add the catalog" (`knowledge.knowledgeBaseAddCatalog(id, catalog)`).
/// Calls: [`existing`], `source_add::add_catalog`, `base_view::screen`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn knowledge_base_add_catalog(
    state: State<'_, AppState>,
    id: i64,
    catalog: String,
) -> Result<BaseScreenView, AppError> {
    let session = state.session()?;
    let db = session.db();

    existing(&db, id)?;
    source_add::add_catalog(&db, id, &catalog)?;

    base_view::screen(&db, id)
}

/// Gives knowledge base `id` the note `text` of the evaluator, named `name`.
///
/// Called by the page: `KnowledgeBase.svelte`, "Add the note" (`knowledge.knowledgeBaseAddNote(id, name, text)`).
/// Calls: [`existing`], `source_add::add_note`, `base_view::screen`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn knowledge_base_add_note(
    state: State<'_, AppState>,
    id: i64,
    name: String,
    text: String,
) -> Result<BaseScreenView, AppError> {
    let session = state.session()?;
    let db = session.db();

    existing(&db, id)?;
    source_add::add_note(&db, id, &name, &text)?;

    base_view::screen(&db, id)
}

/// Removes source `source` of knowledge base `id` with its passages.
///
/// Called by the page: `KnowledgeBase.svelte`, "Remove" of a source (`knowledge.knowledgeBaseRemove(id, source)`).
/// Calls: `db_knowledge::delete::source`, `base_view::screen`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn knowledge_base_remove(state: State<'_, AppState>, id: i64, source: i64) -> Result<BaseScreenView, AppError> {
    let session = state.session()?;
    let db = session.db();

    ensure!(db_knowledge::delete::source(&db, id, source)?, AppError::Invalid("the source does not exist".to_owned()));

    base_view::screen(&db, id)
}

/// The catalogs of OSCAL of the installation, to be given to a base.
///
/// Called by the page: `KnowledgeBase.svelte`, when a base opens (`knowledge.knowledgeBaseCatalogs()`).
/// Calls: `base_view::catalogs`.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn knowledge_base_catalogs(state: State<'_, AppState>) -> Result<Vec<CatalogChoiceView>, AppError> {
    let session = state.session()?;

    base_view::catalogs(&session.db())
}

/// Refused when knowledge base `id` does not exist (before a dialog opens
/// or a source is read for it).
///
/// Called by: the commands that add a source.
fn existing(db: &UserDb, id: i64) -> Result<(), AppError> {
    ensure!(
        db_knowledge::read::base(db, id)?.is_some(),
        AppError::Invalid("the knowledge base does not exist".to_owned())
    );

    Ok(())
}

// ── The search ────────────────────────────────────────────────────────

/// The passages of knowledge base `base` that fit `text`, the best first.
///
/// Called by the page: `KnowledgeSearch.svelte`, "Search" (`knowledge.knowledgeSearchRun(base, text)`).
/// Calls: `base_search::search`, `search_view::all`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn knowledge_search_run(state: State<'_, AppState>, base: i64, text: String) -> Result<Vec<HitView>, AppError> {
    let session = state.session()?;

    Ok(search_view::all(base_search::search(&session.db(), base, &text)?))
}

// ── The agents ────────────────────────────────────────────────────────

/// The agents, by name, with the numbers of their bases.
///
/// Called by the page: `AgentList.svelte`, when the tab Agents opens (`knowledge.agentListGet()`).
/// Calls: `db_knowledge::read::agents`, `agents_view::all`.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn agent_list_get(state: State<'_, AppState>) -> Result<Vec<AgentSummaryView>, AppError> {
    let session = state.session()?;

    Ok(agents_view::all(db_knowledge::read::agents(&session.db())?))
}

/// Makes a new agent named `name`, with the default instructions and
/// limits and no base yet; gives its form, where its bases are ticked.
///
/// Called by the page: `AgentList.svelte`, "Make" (`knowledge.agentListCreate(name)`).
/// Calls: `agent_view::new_agent`, `check_agent`, `db_knowledge::create::agent`, `agent_view::form`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn agent_list_create(state: State<'_, AppState>, name: String) -> Result<AgentFormView, AppError> {
    let session = state.session()?;
    let db = session.db();
    let fields = agent_view::new_agent(&name);

    check_agent(&fields).map_err(|error| AppError::Invalid(error.to_string()))?;
    let id = db_knowledge::create::agent(&db, &fields)?;

    agent_view::form(&db, find::agent(&db, id)?)
}

/// Deletes agent `id` with its trial; its bases stay.
///
/// Called by the page: `AgentList.svelte`, "Delete" (`knowledge.agentListDelete(id)`).
/// Calls: `db_knowledge::delete::agent`, `agents_view::all`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn agent_list_delete(state: State<'_, AppState>, id: i64) -> Result<Vec<AgentSummaryView>, AppError> {
    let session = state.session()?;
    let db = session.db();

    ensure!(db_knowledge::delete::agent(&db, id)?, AppError::Invalid("the agent does not exist".to_owned()));

    Ok(agents_view::all(db_knowledge::read::agents(&db)?))
}

/// The form of agent `id`, with every knowledge base ticked or not.
///
/// Called by the page: `AgentForm.svelte`, when an agent opens (`knowledge.agentFormGet(id)`).
/// Calls: `find::agent`, `agent_view::form`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn agent_form_get(state: State<'_, AppState>, id: i64) -> Result<AgentFormView, AppError> {
    let session = state.session()?;
    let db = session.db();

    agent_view::form(&db, find::agent(&db, id)?)
}

/// Gives agent `id` the fields of `form`: its name, instructions, glossary,
/// limits, and the bases it draws on.
///
/// Called by the page: `AgentForm.svelte`, "Save" (`knowledge.agentFormSave(id, form)`).
/// Calls: `check_agent`, `db_knowledge::update::agent`, `find::agent`, `agent_view::form`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn agent_form_save(state: State<'_, AppState>, id: i64, form: AgentForm) -> Result<AgentFormView, AppError> {
    let session = state.session()?;
    let db = session.db();
    let fields = form.fields();

    check_agent(&fields).map_err(|error| AppError::Invalid(error.to_string()))?;
    ensure!(db_knowledge::update::agent(&db, id, &fields)?, AppError::Invalid("the agent does not exist".to_owned()));

    agent_view::form(&db, find::agent(&db, id)?)
}

/// Writes agent `id` with its knowledge bases, their sources and passages
/// into a file the evaluator chooses in a dialog of the system (step 57.7);
/// `false` when no place was chosen. The page says first that the file
/// holds the text of the sources.
///
/// Called by the page: `AgentForm.svelte`, "Export…" (`knowledge.agentFormExport(id)`).
/// Calls: `agent_export::file_text`, `safe_file_name`, `choose_place` (the dialog).
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn agent_form_export<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    id: i64,
) -> Result<bool, AppError> {
    let session = state.session()?;
    let (name, text) = agent_export::file_text(&session.db(), id)?;

    let Some(path) = choose_place(app, safe_file_name(&name, "agent")).await? else {
        return Ok(false);
    };

    match tokio::fs::write(&path, text).await {
        Ok(()) => Ok(true),
        Err(error) => Err(AppError::Failed(format!("the file was not written: {error}"))),
    }
}

/// Imports an agent from a file the evaluator chooses in a dialog of the
/// system, with its knowledge bases, under free names; gives its form, or
/// `None` when no file was chosen (step 57.7).
///
/// Called by the page: `AgentList.svelte`, "Import an agent…" (`knowledge.agentListImport()`).
/// Calls: `choose_file`, `agent_import::read_file` (on a thread for blocking
/// work), `agent_import::read`, `db_knowledge::create::imported_agent`, `agent_view::form`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn agent_list_import<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<Option<AgentFormView>, AppError> {
    let session = state.session()?;

    let Some(path) = choose_file(app).await? else {
        return Ok(None);
    };

    // spawn_blocking takes a closure: a library place (rule 9 of CLAUDE.md).
    let text = tokio::task::spawn_blocking(move || agent_import::read_file(&path))
        .await
        .map_err(|_| AppError::Failed("the file was not read".to_owned()))??;
    let (fields, bases) = agent_import::read(&text)?;
    let db = session.db();
    let id = db_knowledge::create::imported_agent(&db, &fields, &bases)?;

    Ok(Some(agent_view::form(&db, find::agent(&db, id)?)?))
}

// ── The proving ground ────────────────────────────────────────────────

/// The passages agent `agent` would give the model with `question`, without
/// asking the AI: to tune the agent (its bases, glossary, limits).
///
/// Called by the page: `ProvingGround.svelte`, "Show the passages" (`knowledge.provingGroundFind(agent, question)`).
/// Calls: `find::agent`, `find::passages`, `search_view::all`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn proving_ground_find(state: State<'_, AppState>, agent: i64, question: String) -> Result<Vec<HitView>, AppError> {
    let session = state.session()?;
    let db = session.db();

    Ok(search_view::all(find::passages(&db, &find::agent(&db, agent)?, &question)?))
}

/// The last trial of agent `agent`: the question, the passages, the answer
/// and its quotes checked; `None` before the first.
///
/// Called by the page: `ProvingGround.svelte`, when an agent opens and after a question (`knowledge.provingGroundGet(agent)`).
/// Calls: `db_knowledge::read::trial`, `trial_view::trial`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn proving_ground_get(state: State<'_, AppState>, agent: i64) -> Result<Option<TrialView>, AppError> {
    let session = state.session()?;

    match db_knowledge::read::trial(&session.db(), agent)? {
        Some(trial) => Ok(Some(trial_view::trial(trial))),
        None => Ok(None),
    }
}

/// Asks agent `agent` the question `question` through the AI and keeps the
/// trial; the events of the work arrive through `on_event`.
///
/// Called by the page: `ProvingGround.svelte`, "Ask the agent" (`knowledge.provingGroundAsk(agent, question, onEvent)`).
/// Calls: `AppState::start_work`, `trial_ask::ask`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn proving_ground_ask(
    state: State<'_, AppState>,
    agent: i64,
    question: String,
    on_event: Channel<AiEvent>,
) -> Result<(), AppError> {
    let session = state.session()?;
    let stop = state.start_work();

    trial_ask::ask(&session, agent, &question, &stop, &on_event).await
}

/// Stops the work of the AI of the view, if there is one.
///
/// Called by the page: `ProvingGround.svelte`, "Stop" (`knowledge.knowledgeStop()`).
/// Calls: `AppState::stop_work`.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn knowledge_stop(state: State<'_, AppState>) {
    state.stop_work();
}

#[cfg(test)]
mod tests {
    //! Tests of the commands of the view Knowledge (RAG), called as the page
    //! calls them: by name, through the IPC of Tauri. The dialog of a file is
    //! not opened here; reading a file is tested in `knowledge_base/add.rs`.

    use serde_json::json;

    use crate::test_support::db::basic_catalog;
    use crate::views::test_support::logged_in_window;

    #[test]
    fn a_base_is_made_given_sources_searched_and_deleted() {
        let window = logged_in_window();
        let form = json!({ "name": "Common Criteria", "description": "CC and the CEM." });

        let base = window.call("knowledge_list_create", json!({ "form": form })).unwrap();
        let id = base["id"].as_i64().unwrap();
        assert_eq!(window.call("knowledge_list_create", json!({ "form": form })).unwrap_err()["kind"], "failed");

        let note = json!({ "id": id, "name": "Scheme note", "text": "AVA_VAN.2 asks for a public search of vulnerabilities." });
        let base = window.call("knowledge_base_add_note", note).unwrap();
        assert_eq!(
            base["sources"][0],
            json!({ "id": base["sources"][0]["id"], "kind": "note", "name": "Scheme note", "passages": 1 })
        );

        let hits = window.call("knowledge_search_run", json!({ "base": id, "text": "vulnerability search" })).unwrap();
        assert_eq!(hits[0]["source"], "Scheme note", "the stems of the words are found");

        let list = window.call("knowledge_list_get", json!({})).unwrap();
        assert_eq!((list[0]["sources"].clone(), list[0]["passages"].clone()), (json!(1), json!(1)));

        let source = base["sources"][0]["id"].clone();
        let base = window.call("knowledge_base_remove", json!({ "id": id, "source": source })).unwrap();
        assert_eq!(base["sources"], json!([]));

        assert_eq!(window.call("knowledge_list_delete", json!({ "id": id })).unwrap(), json!([]));
        assert_eq!(window.call("knowledge_base_get", json!({ "id": id })).unwrap_err()["kind"], "invalid");
    }

    #[test]
    fn a_base_is_renamed_and_given_a_catalog() {
        let window = logged_in_window();
        let catalog = basic_catalog();
        crate::db_old::oscal::create::document(&window.session().db(), &catalog).unwrap();
        let base =
            window.call("knowledge_list_create", json!({ "form": { "name": "ISO", "description": "" } })).unwrap();
        let id = base["id"].clone();

        let choices = window.call("knowledge_base_catalogs", json!({})).unwrap();
        assert_eq!(choices[0]["uuid"], catalog.uuid());

        let base = window.call("knowledge_base_add_catalog", json!({ "id": id, "catalog": catalog.uuid() })).unwrap();
        assert_eq!(base["sources"][0]["kind"], "catalog");

        let form = json!({ "name": "ISO 27002", "description": "Controls." });
        let base = window.call("knowledge_base_save", json!({ "id": id, "form": form })).unwrap();
        assert_eq!((base["name"].clone(), base["description"].clone()), (json!("ISO 27002"), json!("Controls.")));

        let empty = json!({ "id": id, "form": { "name": " ", "description": "" } });
        assert_eq!(window.call("knowledge_base_save", empty).unwrap_err()["kind"], "invalid");
        assert_eq!(
            window.call("knowledge_base_add_note", json!({ "id": 99, "name": "", "text": "x" })).unwrap_err()["kind"],
            "invalid"
        );
    }

    #[test]
    fn an_agent_is_joined_to_its_bases_in_its_form_and_finds_their_passages() {
        let window = logged_in_window();
        let base =
            window.call("knowledge_list_create", json!({ "form": { "name": "CC", "description": "" } })).unwrap();
        let base_id = base["id"].clone();
        let note =
            json!({ "id": base_id, "name": "Part 1", "text": "The TOE Security Functionality enforces the SFRs." });
        window.call("knowledge_base_add_note", note).unwrap();

        let form = window.call("agent_list_create", json!({ "name": "CC agent" })).unwrap();
        let id = form["id"].clone();
        assert_eq!(form["bases"][0]["chosen"], false, "a new agent draws on no base yet");

        let saved = json!({ "id": id, "form": {
            "name": "CC agent", "instructions": "You know CC.", "glossary": "TSF = TOE Security Functionality",
            "passages": 4, "characters": 3000, "bases": [base_id]
        } });
        let form = window.call("agent_form_save", saved).unwrap();
        assert_eq!((form["bases"][0]["chosen"].clone(), form["passages"].clone()), (json!(true), json!(4)));

        let hits =
            window.call("proving_ground_find", json!({ "agent": id, "question": "What does the TSF do?" })).unwrap();
        assert_eq!(hits[0]["source"], "Part 1", "found through the glossary");
        assert_eq!(window.call("proving_ground_get", json!({ "agent": id })).unwrap(), json!(null));

        let list = window.call("agent_list_get", json!({})).unwrap();
        assert_eq!(list[0]["bases"], 1);
        window.call("knowledge_list_delete", json!({ "id": base_id })).unwrap();
        assert_eq!(
            window.call("agent_list_get", json!({})).unwrap()[0]["bases"],
            0,
            "a deleted base leaves its agents"
        );

        let wrong = json!({ "id": id, "form": { "name": "CC agent", "instructions": "", "glossary": "TSF",
                                                "passages": 4, "characters": 3000, "bases": [] } });
        assert_eq!(window.call("agent_form_save", wrong).unwrap_err()["kind"], "invalid");
        assert_eq!(window.call("agent_list_delete", json!({ "id": id })).unwrap(), json!([]));
    }
}
