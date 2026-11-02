//! A proof of the laboratory (`LabProof`) given in an evaluation (user,
//! 2026-10-30): its text and the record of its test (when, by whom, with
//! which equipment), in the place the plan reserved for a work unit. It is
//! frozen once given (its store).
//!
//! ```text
//! a text ──► give_proof: a place a work unit of the evaluation needs, its record whole ──► update_lab_proof
//! ```

use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::error::DomainError;
use crate::objects::evaluation::evaluation_plan::evaluation_plan_rules::evaluation::{
    evaluation_in_progress, needed_proofs,
};
use crate::objects::evaluation::lab_proof::lab_proof_model::LabProof;
use crate::objects::evaluation::lab_proof::lab_proof_store::{read_lab_proof, update_lab_proof};
use crate::objects::evaluation::work_unit::work_unit_store::read_work_units;
use crate::objects::text::checked_text;

/// The longest tester or equipment, in characters.
const MAX_FIELD: usize = 500;

/// The longest text of a proof, in characters.
const MAX_PROOF: usize = 1_000_000;

/// Gives the proof `proof.id` of the evaluation `evaluation` its text and
/// the record of its test; its name stays as it was reserved.
///
/// Input: `evaluation`, the id of an evaluation in progress; `proof`, the
/// proof as typed: its id that of a place a work unit needs, its text,
/// date, tester and equipment.
/// Output: nothing; `DomainError::Refused` unless the evaluation is in
/// progress and a work unit of it needs the proof, or for a part of the
/// record missing or too long; its store refuses a proof given already.
///
/// Called by: `panels/views/evaluation/inputs.rs`: `evaluation_proof_give`.
/// Calls: `objects/evaluation/evaluation_plan/evaluation_plan_rules/evaluation.rs`: `evaluation_in_progress`,
/// `needed_proofs`; `objects/text.rs`: `checked_text`; `objects/evaluation/work_unit/work_unit_store.rs`:
/// `read_work_units`; `objects/evaluation/lab_proof/lab_proof_store.rs`: `read_lab_proof`, `update_lab_proof`.
pub fn give_proof(db: &UserDatabase, evaluation: i64, proof: &LabProof) -> Result<(), DomainError> {
    evaluation_in_progress(db, evaluation)?;

    ensure!(
        needed_proofs(&read_work_units(db, evaluation)?).contains(&proof.id),
        DomainError::Refused("the proof is not one of the evaluation".to_owned())
    );
    ensure!(
        proof.performed.is_some_and(|performed| performed > 0),
        DomainError::Refused("the date of the test is missing".to_owned())
    );

    let place = read_lab_proof(db, proof.id)?;
    let given = LabProof {
        id: proof.id,
        name: place.name,
        proof: Some(checked_text(proof.proof.as_deref().unwrap_or_default(), "the text of the proof", MAX_PROOF)?),
        performed: proof.performed,
        tester: Some(checked_text(proof.tester.as_deref().unwrap_or_default(), "the tester", MAX_FIELD)?),
        equipment: Some(checked_text(proof.equipment.as_deref().unwrap_or_default(), "the equipment", MAX_FIELD)?),
    };

    Ok(update_lab_proof(db, &given)?)
}

#[cfg(test)]
mod tests {
    //! Tests of a proof given in an evaluation: once, with its whole record.

    use super::*;

    use crate::test_support::database::database;
    use crate::test_support::evaluation::evaluation_needing_inputs;

    /// The first proof the work units of the evaluation `evaluation` need.
    fn first_proof(db: &UserDatabase, evaluation: i64) -> LabProof {
        let proof = needed_proofs(&read_work_units(db, evaluation).unwrap())[0];

        read_lab_proof(db, proof).unwrap()
    }

    #[test]
    fn a_proof_is_given_once_with_its_whole_record() {
        let (_directory, db) = database();
        let id = evaluation_needing_inputs(&db);
        let place = first_proof(&db, id);
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
            Err(DomainError::Refused(_))
        ));

        give_proof(&db, id, &given).unwrap();

        let stored = read_lab_proof(&db, place.id).unwrap();

        assert_eq!((stored.name.as_str(), stored.tester.as_deref()), ("Scan", Some("K. K.")));
        assert!(matches!(give_proof(&db, id, &given), Err(DomainError::Db(_))));
    }
}
