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

use crate::objects::evaluation::lab_proof::lab_proof_model::LabProof;
use crate::objects::evaluation::lab_proof::lab_proof_rules::give_proof;
use crate::objects::evaluation::product_document::product_document_rules::{document_file, give_document};
use crate::panels::session::Session;
use crate::panels::views::error::ViewError;
use crate::panels::views::opened::database;

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
/// Input: `app`, the handle of the window; `session`, the session of the
/// window, someone logged in; `id`, the id of an evaluation in progress;
/// `document`, the id of one of its documents; `version`, the version typed.
/// Output: whether a file was chosen (`true`); `ViewError::NotLoggedIn` when
/// nobody is logged in, the refusals of the file or the document.
///
/// Called by: `ui/src/lib/api/evaluation.ts`: `evaluationDocumentGive`, from
/// `ui/src/views/evaluation/EvaluationEditor.svelte`, "Give the file".
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`; the dialog of the system;
/// `objects/evaluation/product_document/product_document_rules.rs`: `document_file`, `give_document`.
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

    give_document(database(opened.as_ref())?, id, document, content, &version)?;

    Ok(true)
}

/// Gives the proof `proof` of the evaluation `id` its text `text` and the
/// record of its test: performed at `performed` (seconds since 1970, UTC)
/// by `tester` with `equipment`.
///
/// Input: `session`, the session of the window, someone logged in; `id`, the id
/// of an evaluation in progress; `proof`, the id of a proof one of its work
/// units needs; `text`, `performed`, `tester`, `equipment`, the proof and the
/// record of its test.
/// Output: nothing; `ViewError::NotLoggedIn` when nobody is logged in, the
/// refusals of the proof.
///
/// Called by: `ui/src/lib/api/evaluation.ts`: `evaluationProofGive`, from
/// `ui/src/views/evaluation/EvaluationEditor.svelte`, "Give the proof".
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/evaluation/lab_proof/lab_proof_rules.rs`: `give_proof`.
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

    Ok(give_proof(database(opened.as_ref())?, id, &given)?)
}
