//! The inputs of an evaluation as the panel shows them (user, 2026-10-30):
//! every place of a document of the product and of a proof of the
//! laboratory, given or not, with how many work units need it. Giving them
//! is a rule of the data (`objects/evaluation/product_document/product_document_rules.rs`,
//! `objects/evaluation/lab_proof/lab_proof_rules.rs`).
//!
//! ```text
//! inputs ──► every place, whether it is given, how many work units need it
//! ```

use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::evaluation::evaluation_plan::evaluation_plan_rules::evaluation::needed_proofs;
use crate::objects::evaluation::evaluation_plan::evaluation_plan_store::read_evaluation_plan;
use crate::objects::evaluation::lab_proof::lab_proof_model::LabProof;
use crate::objects::evaluation::lab_proof::lab_proof_store::read_lab_proof;
use crate::objects::evaluation::product_document::product_document_model::ProductDocument;
use crate::objects::evaluation::product_document::product_document_store::read_product_document;
use crate::objects::evaluation::work_unit::work_unit_model::WorkUnit;
use crate::objects::evaluation::work_unit::work_unit_store::read_work_units;
use crate::panels::logic::evaluation::error::EvaluationError;

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

/// How many of the work units `units` need the input `id`.
///
/// Input: `units`, the work units of the evaluation; `id`, the id of a place
/// of an input; `document`, whether it is a document (otherwise a proof).
/// Output: the count (`3`).
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
/// Input: `db`, the open database; `evaluation`, the id of a stored
/// evaluation.
/// Output: the places (`Inputs { documents: [Needed { input: ST, work_units:
/// 12 }], proofs: [...] }`); an error of the database.
///
/// Called by: `panels/views/evaluation/evaluations.rs`: `input_views`.
/// Calls: `objects/evaluation/evaluation_plan/evaluation_plan_store.rs`: `read_evaluation_plan`;
/// `objects/evaluation/work_unit/work_unit_store.rs`: `read_work_units`;
/// `objects/evaluation/product_document/product_document_store.rs`: `read_product_document`;
/// `objects/evaluation/evaluation_plan/evaluation_plan_rules/evaluation.rs`: `needed_proofs`;
/// `objects/evaluation/lab_proof/lab_proof_store.rs`: `read_lab_proof`; [`needing`].
pub fn inputs(db: &UserDatabase, evaluation: i64) -> Result<Inputs, EvaluationError> {
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
    //! Tests of the inputs of an evaluation: each place, with how many work
    //! units need it.

    use super::*;

    use crate::test_support::database::database;
    use crate::test_support::evaluation::evaluation_needing_inputs;

    #[test]
    fn every_place_is_shown_with_the_work_units_that_need_it() {
        let (_directory, db) = database();
        let found = inputs(&db, evaluation_needing_inputs(&db)).unwrap();

        assert_eq!((found.documents.len(), found.documents[0].work_units), (1, 1));
        assert_eq!((found.proofs.len(), found.proofs[0].work_units), (1, 1));
        assert!(found.proofs[0].input.proof.is_none());
    }
}
