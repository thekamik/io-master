//! The entry of the view Knowledge (RAG) (step 57 of docs/todo.md; page:
//! `ui/src/views/knowledge/`, its commands in `ui/src/lib/api/knowledge.ts`):
//! the knowledge bases of the AI. The evaluator makes a base for a standard
//! or a task, gives it sources (a PDF, a text file, a catalog of OSCAL, a
//! note), which are cut into passages and indexed, and searches it. Later
//! stages make agents of the bases and attach them to the works of the AI
//! (57.3 to 57.6). The flow of data: `docs/architecture.md`, section 12.

use tauri::{AppHandle, Runtime, State};

use crate::app::error::AppError;
use crate::app::state::AppState;
use crate::db::knowledge as db_knowledge;
use crate::db::user::database::UserDb;
use crate::model::knowledge::check_base;

use super::knowledge_base::add as source_add;
use super::knowledge_base::dialog::choose_source;
use super::knowledge_base::view::{self as base_view, BaseScreenView, CatalogChoiceView};
use super::knowledge_list::view::{self as list_view, BaseForm, BaseSummaryView};
use super::knowledge_search::search as base_search;
use super::knowledge_search::view::{self as search_view, HitView};

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
        crate::db::oscal::create::document(&window.session().db(), &catalog).unwrap();
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
}
