//! The entry of the component "viewer of a standard": the original page of
//! a PDF, with a quote highlighted in it, drawn by pdf.js in the page (page:
//! `ui/src/components/pdf_viewer/`, its commands in
//! `ui/src/lib/api/components/pdf_viewer.ts`). Used by the view Evaluation
//! plan and by its editor of a catalog. The PDF comes from the encrypted
//! database; the page, the zoom and the selection of a quote are
//! presentation.

use tauri::State;
use tauri::ipc::Response;

use crate::app::error::AppError;
use crate::app::state::AppState;
use crate::db::standards as db_standards;

use super::view::{self, ViewerDocumentView};

// ── When the component opens ──────────────────────────────────────────

/// The documents the viewer can show, by title and part.
///
/// Called by the page: `PdfViewer.svelte`, when it opens
/// (`pdfViewer.pdfViewerDocuments()`).
/// Calls: `db_standards::read::documents`, `view::all`.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn pdf_viewer_documents(state: State<'_, AppState>) -> Result<Vec<ViewerDocumentView>, AppError> {
    let session = state.session()?;

    Ok(view::all(db_standards::read::documents(&session.db())?))
}

/// The PDF of document `id`, as raw bytes.
///
/// Called by the page: `PdfViewer.svelte`, when it opens and when another
/// document is chosen in it (`pdfViewer.pdfViewerGet(id)`).
/// Calls: `db_standards::read::pdf` (the file from the encrypted database).
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn pdf_viewer_get(state: State<'_, AppState>, id: i64) -> Result<Response, AppError> {
    let session = state.session()?;

    let Some(pdf) = db_standards::read::pdf(&session.db(), id)? else {
        return Err(AppError::Invalid("the document does not exist".to_owned()));
    };

    Ok(Response::new(pdf))
}

#[cfg(test)]
mod tests {
    //! Tests of the commands of the viewer of a standard, called as the page
    //! calls them: by name, through the IPC of Tauri.

    use serde_json::json;

    use crate::test_support::standards::load;
    use crate::views::test_support::logged_in_window;

    #[test]
    fn the_pdf_of_a_document_is_given_and_of_no_document_refused() {
        let window = logged_in_window();
        let id = load(&window.session().db(), b"%PDF-1.7 viewer");

        assert_eq!(window.call("pdf_viewer_documents", json!({})).unwrap()[0]["page_count"], 3);
        assert_eq!(window.call_bytes("pdf_viewer_get", json!({ "id": id })).unwrap(), b"%PDF-1.7 viewer");
        assert_eq!(window.call_bytes("pdf_viewer_get", json!({ "id": id + 1 })).unwrap_err()["kind"], "invalid");
    }
}
