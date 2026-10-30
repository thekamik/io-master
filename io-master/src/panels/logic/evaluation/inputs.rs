//! The inputs of an evaluation (user, 2026-10-30): the documents of the
//! product and the proofs of the laboratory, given in the places the plan
//! reserved. Each is frozen once given (`db/`): what the work units are
//! assessed against never changes.
//!
//! ```text
//! a file ──► document_file: .pdf, .txt or .md, within its limit
//!        ──► give_document: a place of the evaluation, the text readable ──► update_product_document
//! a text ──► give_proof: a place a work unit of the evaluation needs, its record whole ──► update_lab_proof
//! inputs ──► every place, whether it is given, how many work units need it
//! ```
//!
//! The file of a document is kept as it was delivered; whether it is a PDF
//! is told by its first bytes (`%PDF-`), so that the model needs no field
//! of the kind.

use std::fs;
use std::path::Path;

use crate::db::database::Database;
use crate::db::evaluation::evaluation_plan::read_evaluation_plan;
use crate::db::evaluation::lab_proof::{read_lab_proof, update_lab_proof};
use crate::db::evaluation::product_document::{read_product_document, update_product_document};
use crate::db::evaluation::work_unit::read_work_units;
use crate::model::evaluation::lab_proof::LabProof;
use crate::model::evaluation::product_document::ProductDocument;
use crate::model::evaluation::work_unit::WorkUnit;
use crate::panels::logic::evaluation::error::EvaluationError;
use crate::panels::logic::evaluation::evaluations::evaluation_in_progress;
use crate::pdf::poppler::{self, MAX_FILE};

/// The largest text file of a document, in bytes.
const MAX_TEXT_FILE: u64 = 64 * 1024 * 1024;

/// The longest version of a document, tester or equipment, in characters.
const MAX_FIELD: usize = 500;

/// The longest text of a proof, in characters.
const MAX_PROOF: usize = 1_000_000;

/// The first bytes of every PDF.
const PDF_SIGNATURE: &[u8] = b"%PDF-";

/// A place of an input and how many work units of the evaluation need it.
pub struct Needed<T> {
    pub input: T,
    pub work_units: usize,
}

/// Every place of input of an evaluation.
pub struct Inputs {
    pub documents: Vec<Needed<ProductDocument>>,
    pub proofs: Vec<Needed<LabProof>>,
}

/// The file `path` of a document of the product, as it is kept: a `.pdf`,
/// `.txt` or `.md` file within its limit; refused for another kind.
///
/// Called by: `panels/views/evaluation/inputs.rs` (a file chosen in the
/// dialog).
pub fn document_file(path: &Path) -> Result<Vec<u8>, EvaluationError> {
    let read = |error: std::io::Error| EvaluationError::Refused(format!("{}: {error}", path.display()));
    let extension = path.extension().and_then(|extension| extension.to_str()).unwrap_or_default().to_lowercase();
    let size = fs::metadata(path).map_err(read)?.len();
    let limit = match extension.as_str() {
        "txt" | "md" => MAX_TEXT_FILE,
        "pdf" => MAX_FILE,
        _ => return Err(EvaluationError::Refused("only .pdf, .txt and .md files are documents".to_owned())),
    };

    ensure!(size <= limit, EvaluationError::Refused(format!("a .{extension} file over {limit} bytes")));

    fs::read(path).map_err(read)
}

/// The text of the file `content` of a document: a PDF through Poppler,
/// anything else as UTF-8; refused when it holds no text.
///
/// Called by: [`give_document`].
/// Calls: `poppler::text`.
pub fn document_text(content: &[u8]) -> Result<String, EvaluationError> {
    let text = if content.starts_with(PDF_SIGNATURE) {
        poppler::text(content)?
    } else {
        String::from_utf8(content.to_vec())
            .map_err(|_| EvaluationError::Refused("the file is neither a PDF nor a text in UTF-8".to_owned()))?
    };

    ensure!(!text.trim().is_empty(), EvaluationError::Refused("the file holds no text".to_owned()));

    Ok(text)
}

/// A text of a record trimmed; refused when it is empty or over `limit`.
///
/// Called by: [`give_document`], [`give_proof`].
fn checked(text: Option<&str>, what: &str, limit: usize) -> Result<String, EvaluationError> {
    let text = text.unwrap_or_default().trim();

    ensure!(!text.is_empty(), EvaluationError::Refused(format!("{what} is missing")));
    ensure!(text.chars().count() <= limit, EvaluationError::Refused(format!("{what} over {limit} characters")));

    Ok(text.to_owned())
}

/// Gives the document `document` of the evaluation `evaluation` its file
/// `content` and its version `version`. Refused unless the evaluation is in
/// progress, the place is one of its documents and the file holds a text;
/// `db/` refuses a document given already.
///
/// Called by: `panels/views/evaluation/inputs.rs` (`evaluation_document_give`).
/// Calls: `evaluation_in_progress`, [`checked`], [`document_text`],
/// `read_product_document`, `update_product_document` (which indexes the
/// text).
pub fn give_document(
    db: &Database,
    evaluation: i64,
    document: i64,
    content: Vec<u8>,
    version: &str,
) -> Result<(), EvaluationError> {
    let stored = evaluation_in_progress(db, evaluation)?;

    ensure!(
        stored.product_documents.contains(&document),
        EvaluationError::Refused("the document is not one of the evaluation".to_owned())
    );

    let version = checked(Some(version), "the version of the document", MAX_FIELD)?;

    let text = document_text(&content)?;
    let place = read_product_document(db, document)?;
    let given = ProductDocument { content: Some(content), version: Some(version), ..place };

    Ok(update_product_document(db, &given, Some(&text))?)
}

/// The proofs the work units `units` need, each once, in the order of the
/// work units.
///
/// Called by: [`give_proof`], [`inputs`].
fn needed_proofs(units: &[WorkUnit]) -> Vec<i64> {
    let mut proofs = Vec::new();

    for unit in units {
        for proof in unit.input_functional.as_deref().unwrap_or_default() {
            if !proofs.contains(proof) {
                proofs.push(*proof);
            }
        }
    }

    proofs
}

/// Gives the proof `proof.id` of the evaluation `evaluation` its text and
/// the record of its test (when, by whom, with which equipment); its name
/// stays as it was reserved. Refused unless the evaluation is in progress
/// and a work unit of it needs the proof, or a part of the record is
/// missing; `db/` refuses a proof given already.
///
/// Called by: `panels/views/evaluation/inputs.rs` (`evaluation_proof_give`).
/// Calls: `evaluation_in_progress`, `read_work_units`, [`needed_proofs`],
/// [`checked`], `read_lab_proof`, `update_lab_proof`.
pub fn give_proof(db: &Database, evaluation: i64, proof: &LabProof) -> Result<(), EvaluationError> {
    evaluation_in_progress(db, evaluation)?;

    ensure!(
        needed_proofs(&read_work_units(db, evaluation)?).contains(&proof.id),
        EvaluationError::Refused("the proof is not one of the evaluation".to_owned())
    );
    ensure!(
        proof.performed.is_some_and(|performed| performed > 0),
        EvaluationError::Refused("the date of the test is missing".to_owned())
    );

    let place = read_lab_proof(db, proof.id)?;
    let given = LabProof {
        id: proof.id,
        name: place.name,
        proof: Some(checked(proof.proof.as_deref(), "the text of the proof", MAX_PROOF)?),
        performed: proof.performed,
        tester: Some(checked(proof.tester.as_deref(), "the tester", MAX_FIELD)?),
        equipment: Some(checked(proof.equipment.as_deref(), "the equipment", MAX_FIELD)?),
    };

    Ok(update_lab_proof(db, &given)?)
}

/// How many of the work units `units` need the input `id`: as a document
/// with `document`, otherwise as a proof.
///
/// Called by: [`inputs`].
fn needing(units: &[WorkUnit], id: i64, document: bool) -> usize {
    let mut count = 0;

    for unit in units {
        let needs = if document {
            unit.input_product_documents.contains(&id)
        } else {
            unit.input_functional.as_deref().unwrap_or_default().contains(&id)
        };

        if needs {
            count += 1;
        }
    }

    count
}

/// Every place of input of the evaluation `evaluation`: its documents and
/// the proofs its work units need, given or not, each with how many work
/// units need it.
///
/// Called by: `panels/views/evaluation/evaluations.rs` (`evaluation_get`).
/// Calls: `read_evaluation_plan`, `read_work_units`, `read_product_document`,
/// [`needed_proofs`], `read_lab_proof`, [`needing`].
pub fn inputs(db: &Database, evaluation: i64) -> Result<Inputs, EvaluationError> {
    let stored = read_evaluation_plan(db, evaluation)?;
    let units = read_work_units(db, evaluation)?;
    let proofs = needed_proofs(&units);
    let mut found = Inputs {
        documents: Vec::with_capacity(stored.product_documents.len()),
        proofs: Vec::with_capacity(proofs.len()),
    };

    for id in &stored.product_documents {
        let work_units = needing(&units, *id, true);

        found.documents.push(Needed { input: read_product_document(db, *id)?, work_units });
    }

    for id in &proofs {
        found.proofs.push(Needed { input: read_lab_proof(db, *id)?, work_units: needing(&units, *id, false) });
    }

    Ok(found)
}

#[cfg(test)]
mod tests {
    //! Tests of the inputs of an evaluation: a document and a proof given
    //! once, in a place of the evaluation, with what they need.

    use super::*;

    use crate::db::ai::agent::create_agent;
    use crate::db::ai::integration::create_integration;
    use crate::db::evaluation::evaluation_plan::create_evaluation_plan;
    use crate::db::evaluation::lab_proof::create_lab_proof;
    use crate::db::evaluation::work_unit::create_work_unit;
    use crate::panels::logic::evaluation::evaluations::new_evaluation;
    use crate::test_support::database::{agent, database, integration, plan, unit};
    use crate::test_support::pdf::pdf;

    /// An evaluation of a plan with one work unit that needs its document
    /// and one proof; gives the evaluation.
    fn evaluation(db: &Database) -> i64 {
        let routers = plan(db);
        let source = create_evaluation_plan(db, &routers).unwrap();
        let scan =
            LabProof { id: 0, name: "Scan".to_owned(), proof: None, performed: None, tester: None, equipment: None };
        let needing = WorkUnit {
            input_product_documents: routers.product_documents.clone(),
            input_functional: Some(vec![create_lab_proof(db, &scan).unwrap()]),
            ..unit(&routers, "ATE_IND.1-1")
        };

        create_work_unit(db, source, &needing).unwrap();

        let general = create_agent(db, &agent(create_integration(db, &integration()).unwrap())).unwrap();

        new_evaluation(db, source, ["R7", "ACME", "K. K."], general).unwrap()
    }

    #[test]
    fn a_document_is_given_once_and_only_with_a_text() {
        let (_directory, db) = database();
        let id = evaluation(&db);
        let document = read_evaluation_plan(&db, id).unwrap().product_documents[0];

        assert!(matches!(give_document(&db, id, document, vec![0xff, 0xfe], "1.0"), Err(EvaluationError::Refused(_))));
        assert!(matches!(give_document(&db, id, 999, b"text".to_vec(), "1.0"), Err(EvaluationError::Refused(_))));

        give_document(&db, id, document, pdf(&["The TOE is a router."]), " 1.3 ").unwrap();

        let given = read_product_document(&db, document).unwrap();

        assert_eq!(given.version.as_deref(), Some("1.3"));
        assert!(document_text(&given.content.unwrap()).unwrap().contains("The TOE is a router."));
        assert!(matches!(give_document(&db, id, document, b"other".to_vec(), "1.4"), Err(EvaluationError::Db(_))));

        let found = inputs(&db, id).unwrap();

        assert_eq!((found.documents.len(), found.documents[0].work_units), (1, 1));
    }

    #[test]
    fn a_proof_is_given_once_with_its_whole_record() {
        let (_directory, db) = database();
        let id = evaluation(&db);
        let place = inputs(&db, id).unwrap().proofs.remove(0).input;
        let given = LabProof {
            proof: Some("22/tcp open ssh".to_owned()),
            performed: Some(1_793_000_000),
            tester: Some("K. K.".to_owned()),
            equipment: Some("nmap 7.95".to_owned()),
            name: "renamed".to_owned(),
            ..place.clone()
        };

        assert!(matches!(
            give_proof(&db, id, &LabProof { tester: None, ..given.clone() }),
            Err(EvaluationError::Refused(_))
        ));

        give_proof(&db, id, &given).unwrap();

        let stored = read_lab_proof(&db, place.id).unwrap();

        assert_eq!((stored.name.as_str(), stored.tester.as_deref()), ("Scan", Some("K. K.")));
        assert!(matches!(give_proof(&db, id, &given), Err(EvaluationError::Db(_))));
    }
}
