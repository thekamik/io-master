//! The inputs of an evaluation, as the page gives them: the file of a
//! document of the product (chosen in the dialog of the system; the page
//! never sees a path), the text of a proof of the laboratory with the
//! record of its test. Each is given once.
//!
//! ```text
//! EvaluationEditor.svelte ──evaluation_document_give(id, document, version)─► the dialog ──► document_file
//!                                                                              ──► give_document
//!                         ──evaluation_proof_give(id, proof, ...)───────────► give_proof
//! ```

use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use crate::model::evaluation::lab_proof::LabProof;
use crate::panels::logic::evaluation::inputs::{document_file, give_document, give_proof};
use crate::panels::views::error::ViewError;
use crate::panels::views::opened::database;
use crate::session::state::Session;

/// The longest version, tester or equipment from the page, in bytes (the
/// logic checks the characters).
const MAX_FIELD: usize = 2048;

/// The longest text of a proof from the page, in bytes.
const MAX_PROOF_BYTES: usize = 4 * 1024 * 1024;

/// Opens the dialog of the system for the file of the document `document`
/// of the evaluation `id` and gives it, with the version `version`; gives
/// whether a file was chosen. Run apart from the window (`async`): the
/// dialog waits for the user, and a PDF takes a moment.
///
/// Called by: `EvaluationEditor.svelte`, "Give the file".
/// Calls: the dialog of the system, `document_file`, `give_document`.
#[tauri::command(async)]
pub fn evaluation_document_give(
    app: AppHandle,
    session: State<'_, Session>,
    id: i64,
    document: i64,
    version: String,
) -> Result<bool, ViewError> {
    ensure!(version.len() <= MAX_FIELD, ViewError::Invalid("the version is too long".to_owned()));

    let Some(file) = app
        .dialog()
        .file()
        .set_title("Give the document")
        .add_filter("PDF, Text or Markdown", &["pdf", "txt", "md"])
        .blocking_pick_file()
    else {
        return Ok(false);
    };
    let path = file.into_path().map_err(|error| ViewError::Invalid(format!("the file chosen: {error}")))?;
    let content = document_file(&path)?;
    let opened = session.lock()?;

    give_document(database(&opened)?, id, document, content, &version)?;

    Ok(true)
}

/// Gives the proof `proof` of the evaluation `id` its text `text` and the
/// record of its test: performed at `performed` (seconds since 1970, UTC)
/// by `tester` with `equipment`.
///
/// Called by: `EvaluationEditor.svelte`, "Give the proof".
/// Calls: `give_proof`.
#[tauri::command]
pub fn evaluation_proof_give(
    session: State<'_, Session>,
    id: i64,
    proof: i64,
    text: String,
    performed: i64,
    tester: String,
    equipment: String,
) -> Result<(), ViewError> {
    ensure!(text.len() <= MAX_PROOF_BYTES, ViewError::Invalid("the text of the proof is too long".to_owned()));
    ensure!(
        tester.len() + equipment.len() <= MAX_FIELD,
        ViewError::Invalid("the tester or the equipment is too long".to_owned())
    );

    let opened = session.lock()?;
    let given = LabProof {
        id: proof,
        name: String::new(),
        proof: Some(text),
        performed: Some(performed),
        tester: Some(tester),
        equipment: Some(equipment),
    };

    Ok(give_proof(database(&opened)?, id, &given)?)
}
