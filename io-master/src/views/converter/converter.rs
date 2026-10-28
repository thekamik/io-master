//! The entry of the view Converter: the conversion of the documents of a
//! standard (PDF) into a catalog of OSCAL with the help of the AI, its only
//! aim (step 50 of docs/history.md; page: `ui/src/views/converter/`, its
//! commands in `ui/src/lib/api/converter.ts`). The PDFs are loaded,
//! searched and shown in the viewer of a standard (`components/pdf_viewer/`);
//! a catalog is made from them and opens in its editor
//! (`catalog_editor/catalog_editor.rs`), where the AI proposes its groups and
//! controls and the evaluator accepts them. The finished documents of OSCAL
//! are used by the view Evaluation plan. Which tab is open, which catalog is
//! open and what the viewer shows are presentation, kept by the page. The
//! files are chosen in the dialogs of the system, opened in Rust: the page
//! never sees a path. The flow of data: `docs/architecture.md`, section 12.

use tauri::ipc::Channel;
use tauri::{AppHandle, Runtime, State};

use crate::app::error::AppError;
use crate::app::state::AppState;
use crate::db::oscal as db_oscal;
use crate::db::standards as db_standards;

use super::catalog_list::view::{self as catalog_view, CatalogView};
use super::document_search::search::{self, MAX_SEARCH};
use super::document_search::view::{self as hit_view, HitView};
use super::new_catalog::made;
use super::new_catalog::view::{self as source_view, SourceView};
use super::standard_documents::dialog::choose_files;
use super::standard_documents::load::{self, MAX_FILES};
use super::standard_documents::stored::{self, DocumentError};
use super::standard_documents::view::{self as document_view, DocumentForm, DocumentView, LoadEvent};

// ── When the view opens ───────────────────────────────────────────────

/// The catalogs of OSCAL of the evaluator, by title, each to be opened in the
/// editor of a catalog.
///
/// Called by the page: `CatalogList.svelte`, when the view opens (the tab
/// Catalogs) and after a change (`converter.catalogListGet()`).
/// Calls: `db_oscal::read::summaries`, `catalog_view::catalogs`.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn catalog_list_get(state: State<'_, AppState>) -> Result<Vec<CatalogView>, AppError> {
    let session = state.session()?;

    Ok(catalog_view::catalogs(db_oscal::read::summaries(&session.db())?))
}

/// The documents of the evaluator, by title and part.
///
/// Called by the page: `StandardDocuments.svelte`, when the tab Documents of
/// standards opens and after a change (`converter.standardDocumentsGet()`).
/// Calls: `db_standards::read::documents`, `document_view::all`.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn standard_documents_get(state: State<'_, AppState>) -> Result<Vec<DocumentView>, AppError> {
    let session = state.session()?;

    Ok(document_view::all(db_standards::read::documents(&session.db())?))
}

// ── The actions of the elements ───────────────────────────────────────

/// The documents of a standard a catalog can come from, by title and
/// part.
///
/// Called by the page: `NewCatalog.svelte`, when its dialog opens
/// (`converter.newCatalogSources()`).
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
/// (`converter.newCatalogCreate(...)`).
/// Calls: `made::create` (the catalog checked and stored),
/// `CatalogView::new`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn new_catalog_create(
    state: State<'_, AppState>,
    title: String,
    version: String,
    sources: Vec<i64>,
) -> Result<CatalogView, AppError> {
    let session = state.session()?;

    Ok(CatalogView::new(made::create(&session.db(), &title, &version, &sources)?))
}

/// Asks for PDFs in a dialog of the system and loads them (`load.rs`);
/// the events arrive through `on_event`. Nothing chosen: nothing happens.
///
/// Called by the page: `StandardDocuments.svelte`, "Load PDFs"
/// (`converter.standardDocumentsLoad(...)`).
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
/// (`converter.standardDocumentsStop()`).
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
/// (`converter.standardDocumentsUpdate(...)`).
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
/// (`converter.standardDocumentsDelete(id)`).
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
/// (`converter.documentSearchRun(...)`).
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
    //! Tests of the commands of the view Converter, called as the page calls
    //! them: by name, through the IPC of Tauri. The loading of PDFs is tested
    //! in `standard_documents/load.rs`: the dialogs of the system cannot run
    //! in the mock window.

    use serde_json::json;

    use crate::db::oscal as db_oscal;
    use crate::model_old::oscal::OscalDocument;
    use crate::test_support::db::basic_catalog;
    use crate::test_support::standards::load;
    use crate::views::test_support::{logged_in_window, logged_out_window};

    #[test]
    fn the_list_holds_the_catalogs_only() {
        let window = logged_in_window();
        let catalog = basic_catalog();
        let profile = OscalDocument::from_json(include_str!("../../../oscal/examples/basic-profile.json")).unwrap();
        let session = window.session();

        db_oscal::create::document(&session.db(), &catalog).unwrap();
        db_oscal::create::document(&session.db(), &profile).unwrap();

        let listed = window.call("catalog_list_get", json!({})).unwrap();

        assert_eq!(listed.as_array().unwrap().len(), 1);
        assert_eq!(listed[0]["uuid"], catalog.uuid());
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

        assert_eq!(made["title"], "CC Part 3");
        assert_eq!(window.call("catalog_list_get", json!({})).unwrap()[0]["uuid"], made["uuid"]);
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
