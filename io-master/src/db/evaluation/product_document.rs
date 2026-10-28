//! The documents of the product (`ProductDocument`: the ST, the IXIT), in
//! the table `product_documents` of the database of the user, never with
//! the standards: create, read, update; never deleted (user, 2026-10-28).
//! The file itself is kept in the database, encrypted with it.
//!
//! ```text
//! product_documents   id, name, content (the file; NULL: the place only reserved), version
//! ```

#![allow(dead_code, reason = "the new database (2026-10-28), not used by the views yet, only by its tests")]

use rusqlite::named_params;

use crate::db::database::Database;
use crate::db::error::DbError;
use crate::model::evaluation::product_document::ProductDocument;

/// The table, made when it is missing (`database.rs`).
pub const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS product_documents (
        id      INTEGER PRIMARY KEY,
        name    TEXT NOT NULL,
        content BLOB,
        version TEXT
    );
";

/// Stores a new document (or the place of one) and gives its id. A new
/// object has no id yet (`id` 0): the database gives it.
///
/// Called by: nothing yet (the views rewritten on `db/`).
pub fn create_product_document(db: &Database, document: &ProductDocument) -> Result<i64, DbError> {
    ensure!(document.id == 0, DbError::Inconsistent("a new document has an id already".to_owned()));
    ensure!(!document.name.is_empty(), DbError::Inconsistent("a document without a name".to_owned()));

    let connection = db.connection();

    connection.execute(
        "INSERT INTO product_documents (name, content, version) VALUES (:name, :content, :version)",
        named_params! { ":name": document.name, ":content": document.content, ":version": document.version },
    )?;

    Ok(connection.last_insert_rowid())
}

/// Reads the document `id`, with its file.
///
/// Called by: nothing yet (the views rewritten on `db/`).
pub fn read_product_document(db: &Database, id: i64) -> Result<ProductDocument, DbError> {
    let connection = db.connection();
    let mut statement =
        connection.prepare("SELECT id, name, content, version FROM product_documents WHERE id = :id")?;
    let mut rows = statement.query(named_params! { ":id": id })?;
    let Some(row) = rows.next()? else {
        return Err(DbError::NoRow { table: "product_documents", id });
    };

    Ok(ProductDocument {
        id: row.get("id")?,
        name: row.get("name")?,
        content: row.get("content")?,
        version: row.get("version")?,
    })
}

/// Writes the document `document.id` anew: the file given in its
/// reserved place, or a correction of its name or version.
///
/// Called by: nothing yet (the views rewritten on `db/`).
pub fn update_product_document(db: &Database, document: &ProductDocument) -> Result<(), DbError> {
    ensure!(!document.name.is_empty(), DbError::Inconsistent("a document without a name".to_owned()));

    let changed = db.connection().execute(
        "UPDATE product_documents SET name = :name, content = :content, version = :version WHERE id = :id",
        named_params! {
            ":id": document.id,
            ":name": document.name,
            ":content": document.content,
            ":version": document.version,
        },
    )?;

    ensure!(changed == 1, DbError::NoRow { table: "product_documents", id: document.id });

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Tests of the documents of the product: the place reserved, the file
    //! given, read back as it was.

    use super::*;

    use crate::test_support::database::database;

    fn place() -> ProductDocument {
        ProductDocument { id: 0, name: "ST".to_owned(), content: None, version: None }
    }

    #[test]
    fn a_place_is_reserved_and_its_file_given() {
        let (_directory, db) = database();
        let id = create_product_document(&db, &place()).unwrap();

        assert_eq!(read_product_document(&db, id).unwrap(), ProductDocument { id, ..place() });

        let given = ProductDocument {
            id,
            content: Some(vec![0x25, 0x50, 0x44, 0x46, 0]),
            version: Some("1.3".to_owned()),
            ..place()
        };

        update_product_document(&db, &given).unwrap();

        assert_eq!(read_product_document(&db, id).unwrap(), given);
    }

    #[test]
    fn a_document_without_a_name_or_missing_is_refused() {
        let (_directory, db) = database();

        assert!(matches!(
            create_product_document(&db, &ProductDocument { name: String::new(), ..place() }),
            Err(DbError::Inconsistent(_))
        ));
        assert!(matches!(read_product_document(&db, 3), Err(DbError::NoRow { .. })));
        assert!(matches!(
            update_product_document(&db, &ProductDocument { id: 3, ..place() }),
            Err(DbError::NoRow { .. })
        ));
    }
}
