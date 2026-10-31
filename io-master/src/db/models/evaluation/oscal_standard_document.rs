//! The OSCAL documents of the standards (`OscalStandardDocument`), in the
//! table `oscal_standard_documents` of the database of the user: create,
//! read, update; never deleted (user, 2026-10-28).
//!
//! ```text
//! oscal_standard_documents   id, title, doc (the OSCAL JSON), version
//! ```

use rusqlite::{Connection, Row, named_params};

use crate::db::error::DbError;
use crate::db::storage::user_database::UserDatabase;
use crate::model::evaluation::oscal_standard_document::OscalStandardDocument;

/// The most standards the user keeps.
pub const MAX_OSCAL_STANDARD_DOCUMENTS: usize = 500;

/// The table, made when it is missing (`schema.rs`, `USER_TABLES`).
pub const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS oscal_standard_documents (
        id      INTEGER PRIMARY KEY,
        title   TEXT NOT NULL,
        doc     TEXT NOT NULL,
        version TEXT NOT NULL
    );
";

/// The columns every read selects, in the order of the model.
const COLUMNS: &str = "id, title, doc, version";

/// Stores a new standard and gives its id. A new object has no id yet
/// (`id` 0): the database gives it.
///
/// Called by: nothing yet (the views rewritten on `db/`).
pub fn create_oscal_standard_document(db: &UserDatabase, document: &OscalStandardDocument) -> Result<i64, DbError> {
    ensure!(document.id == 0, DbError::Inconsistent("a new standard has an id already".to_owned()));

    let connection = db.connection();
    let count: i64 = connection.query_row("SELECT COUNT(*) FROM oscal_standard_documents", [], |row| row.get(0))?;
    let count = usize::try_from(count).map_err(|error| DbError::Inconsistent(error.to_string()))?;

    ensure!(
        count < MAX_OSCAL_STANDARD_DOCUMENTS,
        DbError::Inconsistent(format!("more than {MAX_OSCAL_STANDARD_DOCUMENTS} standards"))
    );

    connection.execute(
        "INSERT INTO oscal_standard_documents (title, doc, version) VALUES (:title, :doc, :version)",
        named_params! { ":title": document.title, ":doc": document.doc, ":version": document.version },
    )?;

    Ok(connection.last_insert_rowid())
}

/// The model of a row of `oscal_standard_documents`.
///
/// Called by: [`read_oscal_standard_document`], [`read_oscal_standard_documents`].
fn document_of(row: &Row<'_>) -> Result<OscalStandardDocument, DbError> {
    Ok(OscalStandardDocument {
        id: row.get("id")?,
        title: row.get("title")?,
        doc: row.get("doc")?,
        version: row.get("version")?,
    })
}

/// Reads the standard `id`.
///
/// Called by: nothing yet (the views rewritten on `db/`).
/// Calls: [`document_of`].
pub fn read_oscal_standard_document(db: &UserDatabase, id: i64) -> Result<OscalStandardDocument, DbError> {
    let connection: &Connection = db.connection();
    let mut statement =
        connection.prepare(&format!("SELECT {COLUMNS} FROM oscal_standard_documents WHERE id = :id"))?;
    let mut rows = statement.query(named_params! { ":id": id })?;
    let Some(row) = rows.next()? else {
        return Err(DbError::NoRow { table: "oscal_standard_documents", id });
    };

    document_of(row)
}

/// Reads every standard, by its title.
///
/// Called by: nothing yet (the views rewritten on `db/`).
/// Calls: [`document_of`].
pub fn read_oscal_standard_documents(db: &UserDatabase) -> Result<Vec<OscalStandardDocument>, DbError> {
    let connection = db.connection();
    let mut statement =
        connection.prepare(&format!("SELECT {COLUMNS} FROM oscal_standard_documents ORDER BY title"))?;
    let mut rows = statement.query([])?;
    let mut documents = Vec::new();

    for _ in 0..=MAX_OSCAL_STANDARD_DOCUMENTS {
        let Some(row) = rows.next()? else {
            return Ok(documents);
        };

        documents.push(document_of(row)?);
    }

    Err(DbError::Inconsistent(format!("more than {MAX_OSCAL_STANDARD_DOCUMENTS} standards in the database")))
}

/// Writes the standard `document.id` anew.
///
/// Called by: nothing yet (the views rewritten on `db/`).
pub fn update_oscal_standard_document(db: &UserDatabase, document: &OscalStandardDocument) -> Result<(), DbError> {
    let changed = db.connection().execute(
        "UPDATE oscal_standard_documents SET title = :title, doc = :doc, version = :version WHERE id = :id",
        named_params! {
            ":id": document.id,
            ":title": document.title,
            ":doc": document.doc,
            ":version": document.version,
        },
    )?;

    ensure!(changed == 1, DbError::NoRow { table: "oscal_standard_documents", id: document.id });

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Tests of the standards: stored, read, changed.

    use super::*;

    use crate::test_support::database::database;

    fn cem() -> OscalStandardDocument {
        OscalStandardDocument {
            id: 0,
            title: "CEM:2022 Revision 1".to_owned(),
            doc: "{\"catalog\": {}}".to_owned(),
            version: "2022 R1".to_owned(),
        }
    }

    #[test]
    fn a_standard_is_stored_read_and_changed() {
        let (_directory, db) = database();
        let id = create_oscal_standard_document(&db, &cem()).unwrap();
        let mut read = read_oscal_standard_document(&db, id).unwrap();

        assert_eq!(read, OscalStandardDocument { id, ..cem() });

        read.version = "2022 R2".to_owned();
        update_oscal_standard_document(&db, &read).unwrap();

        assert_eq!(read_oscal_standard_documents(&db).unwrap(), vec![read]);
    }

    #[test]
    fn a_new_standard_with_an_id_and_a_missing_one_are_refused() {
        let (_directory, db) = database();

        assert!(matches!(
            create_oscal_standard_document(&db, &OscalStandardDocument { id: 7, ..cem() }),
            Err(DbError::Inconsistent(_))
        ));
        assert!(matches!(read_oscal_standard_document(&db, 7), Err(DbError::NoRow { .. })));
        assert!(matches!(
            update_oscal_standard_document(&db, &OscalStandardDocument { id: 7, ..cem() }),
            Err(DbError::NoRow { .. })
        ));
    }
}
