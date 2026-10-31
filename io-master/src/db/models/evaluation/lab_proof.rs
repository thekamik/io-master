//! The proofs of the laboratory (`LabProof`: the results of the functional
//! tests, as text, with the record of the test), in the table `lab_proofs`
//! of the database of the user: create, read, update; never deleted (user,
//! 2026-10-28). **A proof is frozen once its text is given** (user,
//! 2026-10-30), with the record of its test: what the work units were
//! assessed against never changes.
//!
//! ```text
//! lab_proofs   id, name, proof, performed, tester, equipment
//!              (NULL in the last four: the place only reserved)
//! ```

use rusqlite::{Connection, named_params};

use crate::db::error::DbError;
use crate::db::storage::user_database::UserDatabase;
use crate::model::evaluation::lab_proof::LabProof;

/// The table, made when it is missing (`schema.rs`, `USER_TABLES`).
pub const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS lab_proofs (
        id        INTEGER PRIMARY KEY,
        name      TEXT NOT NULL,
        proof     TEXT,
        performed INTEGER,
        tester    TEXT,
        equipment TEXT
    );
";

/// Stores a new proof of the laboratory, or the place of one in a plan (its
/// name only: "results of the scan of the ports"), and gives its id.
///
/// Input: `db`, the open database of the user; `proof`, the new proof, its
/// `id` 0 (the database gives it).
///
/// Output: the id of the proof (6); an error as [`insert_lab_proof`].
///
/// Called by: `panels/logic/evaluation_plan/fill.rs`: `chosen_proofs`;
/// `panels/logic/evaluation_plan/work_units.rs`: `add_proof_place`.
/// Calls: [`insert_lab_proof`].
pub fn create_lab_proof(db: &UserDatabase, proof: &LabProof) -> Result<i64, DbError> {
    insert_lab_proof(db.connection(), proof)
}

/// Writes a new proof and gives its id, inside the transaction of its
/// caller, so that the copy of a plan writes its proofs in its own.
///
/// Input: `connection`, the open database; `proof`, the new proof, its `id`
/// 0 and a name.
///
/// Output: the id of the proof; an error when it has an id or no name.
///
/// Called by: [`create_lab_proof`]; `evaluation_plan.rs`: `copied_places`.
pub(in crate::db) fn insert_lab_proof(connection: &Connection, proof: &LabProof) -> Result<i64, DbError> {
    ensure!(proof.id == 0, DbError::Inconsistent("a new proof has an id already".to_owned()));
    ensure!(!proof.name.is_empty(), DbError::Inconsistent("a proof without a name".to_owned()));

    connection.execute(
        "INSERT INTO lab_proofs (name, proof, performed, tester, equipment)
         VALUES (:name, :proof, :performed, :tester, :equipment)",
        named_params! {
            ":name": proof.name,
            ":proof": proof.proof,
            ":performed": proof.performed,
            ":tester": proof.tester,
            ":equipment": proof.equipment,
        },
    )?;

    Ok(connection.last_insert_rowid())
}

/// Reads the proof `id`: its name, and its text with the record of its test
/// once given.
///
/// Input: `db`, the open database of the user; `id`, the id of the proof (6).
///
/// Output: the proof (its text `None` while the place is only reserved);
/// `DbError::NoRow` when there is none.
///
/// Called by: [`update_lab_proof`]; `evaluation_plan.rs`: `copied_places`;
/// `panels/logic/evaluation/inputs.rs`: `give_proof`, `inputs`;
/// `panels/logic/evaluation/question.rs`: `proofs_text`, `unit_json`;
/// `panels/logic/evaluation/run.rs`: `missing_inputs`;
/// `panels/logic/evaluation_plan/fill.rs`: `current_parameters`,
/// `proof_named`; `panels/views/evaluation/work_units.rs`: `needs_of`;
/// `panels/views/evaluation_plan/work_units.rs`: `work_unit_get`.
pub fn read_lab_proof(db: &UserDatabase, id: i64) -> Result<LabProof, DbError> {
    let connection = db.connection();
    let mut statement =
        connection.prepare("SELECT id, name, proof, performed, tester, equipment FROM lab_proofs WHERE id = :id")?;
    let mut rows = statement.query(named_params! { ":id": id })?;
    let Some(row) = rows.next()? else {
        return Err(DbError::NoRow { table: "lab_proofs", id });
    };

    Ok(LabProof {
        id: row.get("id")?,
        name: row.get("name")?,
        proof: row.get("proof")?,
        performed: row.get("performed")?,
        tester: row.get("tester")?,
        equipment: row.get("equipment")?,
    })
}

/// Writes the proof `proof.id` anew: its text and the record of its test
/// given in its reserved place, or a correction of the place while it has
/// no text. Refused once the text is given: the proof is then frozen.
///
/// Input: `db`, the open database of the user; `proof`, the proof as it is
/// to be kept, its `id` that of a stored one, a name.
///
/// Output: nothing; an error when it has no name, when its text is given
/// already, or `DbError::NoRow` when there is no proof `proof.id`.
///
/// Called by: `panels/logic/evaluation/inputs.rs`: `give_proof`.
/// Calls: [`read_lab_proof`].
pub fn update_lab_proof(db: &UserDatabase, proof: &LabProof) -> Result<(), DbError> {
    ensure!(!proof.name.is_empty(), DbError::Inconsistent("a proof without a name".to_owned()));

    let stored = read_lab_proof(db, proof.id)?;

    ensure!(
        stored.proof.is_none(),
        DbError::Inconsistent(format!("the proof {} is given already: it is never changed", stored.name))
    );

    let changed = db.connection().execute(
        "UPDATE lab_proofs SET name = :name, proof = :proof, performed = :performed, tester = :tester,
                               equipment = :equipment
         WHERE id = :id",
        named_params! {
            ":id": proof.id,
            ":name": proof.name,
            ":proof": proof.proof,
            ":performed": proof.performed,
            ":tester": proof.tester,
            ":equipment": proof.equipment,
        },
    )?;

    ensure!(changed == 1, DbError::NoRow { table: "lab_proofs", id: proof.id });

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Tests of the proofs of the laboratory: the place reserved, the proof
    //! given with the record of its test and never changed again.

    use super::*;

    use crate::test_support::database::database;

    fn place() -> LabProof {
        LabProof {
            id: 0,
            name: "Results of the scan of the ports".to_owned(),
            proof: None,
            performed: None,
            tester: None,
            equipment: None,
        }
    }

    #[test]
    fn a_place_is_reserved_and_its_proof_given() {
        let (_directory, db) = database();
        let id = create_lab_proof(&db, &place()).unwrap();

        assert_eq!(read_lab_proof(&db, id).unwrap(), LabProof { id, ..place() });

        let given = LabProof {
            id,
            proof: Some("22/tcp open ssh".to_owned()),
            performed: Some(1_793_000_000),
            tester: Some("K. K.".to_owned()),
            equipment: Some("nmap 7.95, laptop LAB-014".to_owned()),
            ..place()
        };

        update_lab_proof(&db, &given).unwrap();

        assert_eq!(read_lab_proof(&db, id).unwrap(), given);
        assert!(matches!(
            update_lab_proof(&db, &LabProof { tester: Some("someone else".to_owned()), ..given.clone() }),
            Err(DbError::Inconsistent(_))
        ));
        assert_eq!(read_lab_proof(&db, id).unwrap(), given);
    }

    #[test]
    fn a_proof_without_a_name_or_missing_is_refused() {
        let (_directory, db) = database();

        assert!(matches!(
            create_lab_proof(&db, &LabProof { name: String::new(), ..place() }),
            Err(DbError::Inconsistent(_))
        ));
        assert!(matches!(read_lab_proof(&db, 5), Err(DbError::NoRow { .. })));
        assert!(matches!(update_lab_proof(&db, &LabProof { id: 5, ..place() }), Err(DbError::NoRow { .. })));
    }
}
