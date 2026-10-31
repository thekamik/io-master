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

use crate::db::models::evaluation::oscal_standard_document::read_oscal_standard_documents;
use crate::panels::logic::evaluation_plan::oscal::MAX_DOCUMENT;
use crate::panels::logic::evaluation_plan::standards::import_standard;
use crate::panels::views::error::ViewError;
use crate::panels::views::opened::database;
use crate::session::state::Session;

/// A standard, as the page lists it.
#[derive(Serialize)]
pub struct StandardView {
    id: i64,
    title: String,
    version: String,
}

/// Every standard of the user, by its title.
///
/// Called by: `EvaluationPlan.svelte`, when the panel opens and after an import.
/// Calls: `read_oscal_standard_documents`.
#[tauri::command]
pub fn standard_list(session: State<'_, Session>) -> Result<Vec<StandardView>, ViewError> {
    let opened = session.lock()?;
    let mut standards = Vec::new();

    for standard in read_oscal_standard_documents(database(&opened)?)? {
        standards.push(StandardView { id: standard.id, title: standard.title, version: standard.version });
    }

    Ok(standards)
}

/// Opens the dialog of the system for an OSCAL catalog (JSON), reads the
/// file within its limit and imports it. Gives its id; `None` when the
/// dialog was closed. Run apart from the window (`async`): the dialog waits
/// for the user.
///
/// Called by: `EvaluationPlan.svelte`, "Import a standard".
/// Calls: the dialog of the system, `import_standard`.
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

    Ok(Some(import_standard(database(&opened)?, &text)?))
}
