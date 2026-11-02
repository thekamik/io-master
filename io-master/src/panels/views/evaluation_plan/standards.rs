//! The standards of the panel of the plan, as the page calls them: their
//! list, and the import of an OSCAL catalog chosen in the dialog of the
//! system (the page never sees a path).
//!
//! ```text
//! EvaluationPlan.svelte ──standard_list()────► read_oscal_standard_documents ──► [StandardView]
//!                       ──standard_import()──► the dialog ──► the file ──► import_standard ──► id
//! ```

use std::fs;

use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use crate::objects::evaluation::oscal_standard_document::oscal_standard_document_rules::MAX_DOCUMENT;
use crate::objects::evaluation::oscal_standard_document::oscal_standard_document_rules::import_standard;
use crate::objects::evaluation::oscal_standard_document::oscal_standard_document_store::read_oscal_standard_documents;
use crate::panels::session::Session;
use crate::panels::views::error::ViewError;
use crate::panels::views::opened::database;

/// A standard, as the page lists it.
#[derive(Serialize)]
pub struct StandardView {
    id: i64,
    title: String,
    version: String,
}

/// Every standard of the user, by its title.
///
/// Input: `session`, the session of the window, someone logged in.
/// Output: the standards (`[{"id": 2, "title": "CEM", "version": "3.1 R5"}]`);
/// `ViewError::NotLoggedIn` when nobody is logged in, an error of the database.
///
/// Called by: `ui/src/lib/api/evaluation_plan.ts`: `standardList`, from
/// `ui/src/views/evaluation_plan/EvaluationPlan.svelte`, when the panel opens and after an import.
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/evaluation/oscal_standard_document/oscal_standard_document_store.rs`: `read_oscal_standard_documents`.
#[tauri::command]
pub fn standard_list(session: State<'_, Session>) -> Result<Vec<StandardView>, ViewError> {
    let opened = session.lock()?;
    let mut standards = Vec::new();

    for standard in read_oscal_standard_documents(database(opened.as_ref())?)? {
        standards.push(StandardView { id: standard.id, title: standard.title, version: standard.version });
    }

    Ok(standards)
}

/// Opens the dialog of the system for an OSCAL catalog (JSON), reads the
/// file within its limit and imports it. Gives its id; `None` when the
/// dialog was closed. Run apart from the window (`async`): the dialog waits
/// for the user.
///
/// Input: `app`, the handle of the window; `session`, the session of the
/// window, someone logged in.
/// Output: the id of the standard (`Some(2)`), `None` when the dialog was
/// closed; `ViewError::NotLoggedIn` when nobody is logged in,
/// `ViewError::Invalid` for a file refused, the refusals of the catalog.
///
/// Called by: `ui/src/lib/api/evaluation_plan.ts`: `standardImport`, from
/// `ui/src/views/evaluation_plan/EvaluationPlan.svelte`, "Import a standard".
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`; the dialog of the system;
/// `objects/evaluation/oscal_standard_document/oscal_standard_document_rules.rs`: `import_standard`.
#[tauri::command(async)]
pub fn standard_import(app: AppHandle, session: State<'_, Session>) -> Result<Option<i64>, ViewError> {
    let Some(file) =
        app.dialog().file().set_title("Import a standard").add_filter("OSCAL catalog", &["json"]).blocking_pick_file()
    else {
        return Ok(None);
    };
    let path = file.into_path().map_err(|error| ViewError::Invalid(format!("the file chosen: {error}")))?;
    let size = fs::metadata(&path).map_err(|error| ViewError::Invalid(format!("{}: {error}", path.display())))?.len();

    let size = usize::try_from(size).map_err(|error| ViewError::Invalid(error.to_string()))?;

    ensure!(size <= MAX_DOCUMENT, ViewError::Invalid(format!("the file is over {MAX_DOCUMENT} bytes")));

    let text = fs::read_to_string(&path).map_err(|error| ViewError::Invalid(format!("{}: {error}", path.display())))?;
    let opened = session.lock()?;

    Ok(Some(import_standard(database(opened.as_ref())?, &text)?))
}
