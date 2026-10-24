//! The entry of the view Evaluation plan: it works only on the documents of
//! OSCAL (page: `ui/src/views/evaluation_plan/`, its commands in
//! `ui/src/lib/api/evaluation_plan.ts`): the list, the import, the export
//! and the deletion now; later the document shown graphically and turned
//! into the flow of data of the evaluation (step 50 of docs/todo.md). The
//! documents are made from the PDFs of a standard in the view Converter
//! (`views/converter/`). The files are chosen in the dialogs of the system,
//! opened in Rust: the page never sees a path. The flow of data:
//! `docs/architecture.md`, section 12.

use tauri::{AppHandle, Runtime, State};

use crate::app::error::AppError;
use crate::app::state::AppState;
use crate::db::oscal as db_oscal;

use super::oscal_documents::dialog::{choose_file, choose_place};
use super::oscal_documents::file::{self, read_file, safe_name};
use super::oscal_documents::view::{self as oscal_view, OscalDocumentView};

// ── When the view opens ───────────────────────────────────────────────

/// The documents of OSCAL of the evaluator, by model and title.
///
/// Called by the page: `OscalDocuments.svelte`, when the view opens and after
/// a change (`evaluation_plan.oscalDocumentsGet()`).
/// Calls: `db_oscal::read::summaries`, `oscal_view::all`.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn oscal_documents_get(state: State<'_, AppState>) -> Result<Vec<OscalDocumentView>, AppError> {
    let session = state.session()?;

    Ok(oscal_view::all(db_oscal::read::summaries(&session.db())?))
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

#[cfg(test)]
mod tests {
    //! Tests of the commands of the view Evaluation plan, called as the page
    //! calls them: by name, through the IPC of Tauri. The import and the
    //! export are tested in `oscal_documents/file.rs`: the dialogs of the
    //! system cannot run in the mock window.

    use serde_json::json;

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
    fn the_documents_need_a_login() {
        assert_eq!(logged_out_window().call("oscal_documents_get", json!({})).unwrap_err()["kind"], "not_logged_in");
    }
}
