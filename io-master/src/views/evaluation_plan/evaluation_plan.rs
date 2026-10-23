//! The entry of the view Evaluation plan: the documents of the standards
//! in NIST OSCAL and the documents of standards (PDF) they quote (page:
//! `ui/src/views/evaluation_plan/`, its commands in
//! `ui/src/lib/api/evaluation_plan.ts`). Which tab is open, which catalog
//! is open and what the viewer shows are presentation, kept by the page; a
//! catalog opens in its editor (`catalog_editor/catalog_editor.rs`), the
//! viewer of a standard is a component (`components/pdf_viewer/`). The
//! files are chosen in the dialogs of the system, opened in Rust: the page
//! never sees a path. The flow of data: `docs/architecture.md`, section 12.

use tauri::ipc::Channel;
use tauri::{AppHandle, Runtime, State};

use crate::app::error::AppError;
use crate::app::state::AppState;
use crate::db::oscal as db_oscal;
use crate::db::standards as db_standards;

use super::document_search::search::{self, MAX_SEARCH};
use super::document_search::view::{self as hit_view, HitView};
use super::new_catalog::made;
use super::new_catalog::view::{self as source_view, SourceView};
use super::oscal_documents::dialog::{choose_file, choose_place};
use super::oscal_documents::file::{self, read_file, safe_name};
use super::oscal_documents::view::{self as oscal_view, OscalDocumentView};
use super::standard_documents::dialog::choose_files;
use super::standard_documents::load::{self, MAX_FILES};
use super::standard_documents::stored::{self, DocumentError};
use super::standard_documents::view::{self as document_view, DocumentForm, DocumentView, LoadEvent};

// ── When the view opens ───────────────────────────────────────────────

/// The documents of OSCAL of the evaluator, by model and title.
///
/// Called by the page: `OscalDocuments.svelte`, when the view opens (the tab
/// OSCAL) and after a change (`evaluation_plan.oscalDocumentsGet()`).
/// Calls: `db_oscal::read::summaries`, `oscal_view::all`.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn oscal_documents_get(state: State<'_, AppState>) -> Result<Vec<OscalDocumentView>, AppError> {
    let session = state.session()?;

    Ok(oscal_view::all(db_oscal::read::summaries(&session.db())?))
}

/// The documents of the evaluator, by title and part.
///
/// Called by the page: `StandardDocuments.svelte`, when the tab Documents of
/// standards opens and after a change (`evaluation_plan.standardDocumentsGet()`).
/// Calls: `db_standards::read::documents`, `document_view::all`.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn standard_documents_get(state: State<'_, AppState>) -> Result<Vec<DocumentView>, AppError> {
    let session = state.session()?;

    Ok(document_view::all(db_standards::read::documents(&session.db())?))
}

// ── The actions of the elements ───────────────────────────────────────

/// Asks for a file of OSCAL (JSON) in a dialog of the system and imports
/// it; `None` when no file was chosen.
///
/// Called by the page: `OscalDocuments.svelte`, "Import an OSCAL document"
/// (`evaluation_plan.oscalDocumentsImport()`).
/// Calls: `choose_file` (the dialog), `read_file` (at most `MAX_FILE`),
/// `file::import` (checked against the schema of NIST, stored).
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn oscal_documents_import<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<Option<OscalDocumentView>, AppError> {
    let session = state.session()?;

    let Some(path) = choose_file(app).await? else {
        return Ok(None);
    };

    let text = read_file(path).await?;
    let summary = file::import(&session.db(), &text)?;

    Ok(Some(OscalDocumentView::new(summary)))
}

/// Asks where to write document `uuid` in a dialog of the system (the
/// name proposed: `file_name`) and writes it; `false` when no place was
/// chosen.
///
/// Called by the page: `OscalDocuments.svelte`, "Export"
/// (`evaluation_plan.oscalDocumentsExport(...)`).
/// Calls: `file::export`, `safe_name`, `choose_place` (the dialog),
/// `tokio::fs::write`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn oscal_documents_export<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    uuid: String,
    file_name: String,
) -> Result<bool, AppError> {
    let session = state.session()?;
    let text = file::export(&session.db(), &uuid)?;

    let Some(path) = choose_place(app, safe_name(&file_name)).await? else {
        return Ok(false);
    };

    match tokio::fs::write(&path, text).await {
        Ok(()) => Ok(true),
        Err(error) => Err(AppError::Failed(format!("the file was not written: {error}"))),
    }
}

/// Deletes document `uuid` with its evaluation support.
///
/// Called by the page: `OscalDocuments.svelte`, "Delete"
/// (`evaluation_plan.oscalDocumentsDelete(uuid)`).
/// Calls: `file::delete` (the document and its evaluation support).
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn oscal_documents_delete(state: State<'_, AppState>, uuid: String) -> Result<(), AppError> {
    let session = state.session()?;

    ensure!(file::delete(&session.db(), &uuid)?, AppError::Invalid("the OSCAL document does not exist".to_owned()));

    Ok(())
}

/// The documents of a standard a catalog can come from, by title and
/// part.
///
/// Called by the page: `NewCatalog.svelte`, when its dialog opens
/// (`evaluation_plan.newCatalogSources()`).
/// Calls: `db_standards::read::documents`, `source_view::all`.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn new_catalog_sources(state: State<'_, AppState>) -> Result<Vec<SourceView>, AppError> {
    let session = state.session()?;

    Ok(source_view::all(db_standards::read::documents(&session.db())?))
}

/// Makes a new, empty catalog named `title`, of `version`, from the
/// documents of a standard `sources`.
///
/// Called by the page: `NewCatalog.svelte`, "Make"
/// (`evaluation_plan.newCatalogCreate(...)`).
/// Calls: `made::create` (the catalog checked and stored),
/// `OscalDocumentView::new`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn new_catalog_create(
    state: State<'_, AppState>,
    title: String,
    version: String,
    sources: Vec<i64>,
) -> Result<OscalDocumentView, AppError> {
    let session = state.session()?;

    Ok(OscalDocumentView::new(made::create(&session.db(), &title, &version, &sources)?))
}

/// Asks for PDFs in a dialog of the system and loads them (`load.rs`);
/// the events arrive through `on_event`. Nothing chosen: nothing happens.
///
/// Called by the page: `StandardDocuments.svelte`, "Load PDFs"
/// (`evaluation_plan.standardDocumentsLoad(...)`).
/// Calls: `choose_files` (the dialog), `AppState::start_work` (the stop
/// signal), `load::load` (the PDFs read part by part, stored).
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn standard_documents_load<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    on_event: Channel<LoadEvent>,
) -> Result<(), AppError> {
    let session = state.session()?;
    let paths = choose_files(app).await?;

    ensure!(paths.len() <= MAX_FILES, AppError::Invalid(format!("at most {MAX_FILES} files can be loaded at once")));

    if paths.is_empty() {
        return Ok(());
    }

    let stop = state.start_work();

    ensure!(
        load::load(&session, &paths, &stop, &on_event).await,
        AppError::Failed("the window can no longer be reached".to_owned())
    );

    Ok(())
}

/// Stops the loading, if one runs.
///
/// Called by the page: `StandardDocuments.svelte`, "Stop"
/// (`evaluation_plan.standardDocumentsStop()`).
/// Calls: `AppState::stop_work`.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn standard_documents_stop(state: State<'_, AppState>) {
    state.stop_work();
}

/// Changes the title, part and version of document `id`.
///
/// Called by the page: `StandardDocuments.svelte`, "Save" of a document
/// (`evaluation_plan.standardDocumentsUpdate(...)`).
/// Calls: `stored::update` (the rules of the fields),
/// `db_standards::read::document`, `DocumentView::new`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn standard_documents_update(
    state: State<'_, AppState>,
    id: i64,
    document: DocumentForm,
) -> Result<DocumentView, AppError> {
    let session = state.session()?;
    let db = session.db();

    stored::update(&db, id, &document.into_fields())?;

    let Some(document) = db_standards::read::document(&db, id)? else {
        return Err(DocumentError::NotFound.into());
    };

    Ok(DocumentView::new(document))
}

/// Deletes document `id`; refused while an OSCAL document quotes it.
///
/// Called by the page: `StandardDocuments.svelte`, "Delete"
/// (`evaluation_plan.standardDocumentsDelete(id)`).
/// Calls: `stored::delete` (refused while an OSCAL document quotes it).
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn standard_documents_delete(state: State<'_, AppState>, id: i64) -> Result<(), AppError> {
    let session = state.session()?;

    stored::delete(&session.db(), id)?;

    Ok(())
}

/// Searches the text of the documents (all, or `document`).
///
/// Called by the page: `DocumentSearch.svelte`, "Search"
/// (`evaluation_plan.documentSearchRun(...)`).
/// Calls: `search::search`, `db_standards::read::documents`, `hit_view::all`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn document_search_run(
    state: State<'_, AppState>,
    text: String,
    document: Option<i64>,
) -> Result<Vec<HitView>, AppError> {
    let session = state.session()?;

    ensure!(
        text.chars().count() <= MAX_SEARCH,
        AppError::Invalid(format!("search for at most {MAX_SEARCH} characters"))
    );

    let db = session.db();

    Ok(hit_view::all(search::search(&db, &text, document)?, &db_standards::read::documents(&db)?))
}

#[cfg(test)]
mod tests {
    //! Tests of the commands of the view Evaluation plan, called as the page
    //! calls them: by name, through the IPC of Tauri. The import and the
    //! export are tested in `oscal_documents/file.rs`, the loading of PDFs
    //! in `standard_documents/load.rs`: the dialogs of the system cannot
    //! run in the mock window.

    use serde_json::json;

    use crate::test_support::standards::load;
    use crate::views::evaluation_plan::oscal_documents::file;
    use crate::views::test_support::{logged_in_window, logged_out_window};

    #[test]
    fn a_document_is_listed_and_deleted() {
        let window = logged_in_window();
        let catalog = include_str!("../../../oscal/examples/basic-catalog.json");
        let uuid = file::import(&window.session().db(), catalog).unwrap().uuid;

        let listed = window.call("oscal_documents_get", json!({})).unwrap();
        assert_eq!(listed[0]["uuid"], uuid);
        assert_eq!(listed[0]["model"], "catalog");

        window.call("oscal_documents_delete", json!({ "uuid": uuid })).unwrap();
        assert_eq!(window.call("oscal_documents_get", json!({})).unwrap(), json!([]));
        assert_eq!(window.call("oscal_documents_delete", json!({ "uuid": uuid })).unwrap_err()["kind"], "invalid");
    }

    #[test]
    fn a_document_is_listed_renamed_and_deleted() {
        let window = logged_in_window();
        let id = load(&window.session().db(), b"%PDF a");

        assert_eq!(window.call("standard_documents_get", json!({})).unwrap()[0]["page_count"], 3);

        let form = json!({ "title": " Common Criteria ", "part": "Part 3", "version": "CC:2022" });
        let changed = window.call("standard_documents_update", json!({ "id": id, "document": form })).unwrap();
        assert_eq!(changed["title"], "Common Criteria");
        assert_eq!(changed["part"], "Part 3");

        window.call("standard_documents_delete", json!({ "id": id })).unwrap();
        assert_eq!(window.call("standard_documents_get", json!({})).unwrap(), json!([]));
        assert_eq!(window.call("standard_documents_delete", json!({ "id": id })).unwrap_err()["kind"], "invalid");
    }

    #[test]
    fn a_title_is_needed() {
        let window = logged_in_window();
        let id = load(&window.session().db(), b"%PDF b");
        let form = json!({ "title": "  ", "part": "", "version": "" });

        let error = window.call("standard_documents_update", json!({ "id": id, "document": form })).unwrap_err();

        assert_eq!(error["kind"], "invalid");
    }

    #[test]
    fn the_documents_need_a_login() {
        assert_eq!(logged_out_window().call("standard_documents_get", json!({})).unwrap_err()["kind"], "not_logged_in");
    }

    #[test]
    fn a_catalog_is_made_from_documents() {
        let window = logged_in_window();
        let source = load(&window.session().db(), b"%PDF catalog");

        assert_eq!(window.call("new_catalog_sources", json!({})).unwrap()[0]["id"], source);

        let made = window
            .call("new_catalog_create", json!({ "title": "CC Part 3", "version": "CC:2022", "sources": [source] }))
            .unwrap();

        assert_eq!(made["model"], "catalog");
        assert_eq!(made["title"], "CC Part 3");
        assert_eq!(window.call("oscal_documents_get", json!({})).unwrap()[0]["uuid"], made["uuid"]);
    }

    #[test]
    fn a_source_that_does_not_exist_is_refused() {
        let window = logged_in_window();

        let error =
            window.call("new_catalog_create", json!({ "title": "X", "version": "", "sources": [99] })).unwrap_err();

        assert_eq!(error["kind"], "invalid");
    }

    #[test]
    fn the_hits_name_their_document_and_page() {
        let window = logged_in_window();
        let id = load(&window.session().db(), b"%PDF c");

        let hits = window.call("document_search_run", json!({ "text": "adv_fsp.4", "document": null })).unwrap();

        assert_eq!(hits[0]["document_id"], id);
        assert_eq!(hits[0]["document"], "Common Criteria — Part 3");
        assert_eq!(hits[0]["page"], 2);
    }

    #[test]
    fn a_too_long_text_is_refused() {
        let window = logged_in_window();
        let text = "x".repeat(1001);

        let error = window.call("document_search_run", json!({ "text": text, "document": null })).unwrap_err();

        assert_eq!(error["kind"], "invalid");
    }
}
