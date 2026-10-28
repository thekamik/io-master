//! The proofs of the laboratory (`LabProof`: the results of the functional
//! tests, as text, with the record of the test), in the table `lab_proofs`
//! of the database of the user: create, read, update; never deleted (user,
//! 2026-10-28).
//!
//! ```text
//! lab_proofs   id, name, proof, performed, tester, equipment
//!              (NULL in the last four: the place only reserved)
//! ```

#![allow(dead_code, reason = "the new database (2026-10-28), not used by the views yet, only by its tests")]

use rusqlite::named_params;

use crate::db::database::Database;
use crate::db::error::DbError;
use crate::model::evaluation::lab_proof::LabProof;

/// The table, made when it is missing (`database.rs`).
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

/// Stores a new proof (or the place of one) and gives its id. A new
/// object has no id yet (`id` 0): the database gives it.
///
/// Called by: nothing yet (the views rewritten on `db/`).
pub fn create_lab_proof(db: &Database, proof: &LabProof) -> Result<i64, DbError> {
    ensure!(proof.id == 0, DbError::Inconsistent("a new proof has an id already".to_owned()));
    ensure!(!proof.name.is_empty(), DbError::Inconsistent("a proof without a name".to_owned()));

    let connection = db.connection();

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

/// Reads the proof `id`.
///
/// Called by: nothing yet (the views rewritten on `db/`).
pub fn read_lab_proof(db: &Database, id: i64) -> Result<LabProof, DbError> {
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

/// Writes the proof `proof.id` anew: the text and the record of the test
/// given in its reserved place, or a correction.
///
/// Called by: nothing yet (the views rewritten on `db/`).
pub fn update_lab_proof(db: &Database, proof: &LabProof) -> Result<(), DbError> {
    ensure!(!proof.name.is_empty(), DbError::Inconsistent("a proof without a name".to_owned()));

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
    //! given with the record of its test.

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
