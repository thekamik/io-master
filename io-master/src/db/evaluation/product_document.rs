//! The documents of the product (`ProductDocument`: the ST, the IXIT), in
//! the table `product_documents` of the database of the user, never with
//! the standards: create, read, update; never deleted (user, 2026-10-28).
//! The file itself is kept in the database, encrypted with it. **A document
//! is frozen once its file is given** (user, 2026-10-30): what the work
//! units were assessed against, and what their quotes are checked in, never
//! changes; a wrong file is mended by a new evaluation.
//!
//! With them the index of their text, which is no model: when the file is
//! given, its text (read by the caller: a PDF through Poppler) is cut into
//! passages, as the knowledge is (`db/ai/rag.rs`), and indexed by FTS5 with
//! their places, in the same transaction. The passages give the text whole
//! again ([`read_document_text`]: for the question of a work unit and the
//! check of its quotes, without reading the PDF again), and those that fit
//! a work unit best ([`read_document_passages`]: when the whole text does
//! not fit into the context of the model).
//!
//! ```text
//! product_documents          id, name, content (the file; NULL: the place only reserved), version
//! product_document_passages  the index (FTS5): text, document, position; made from the text of the file
//! ```

use rusqlite::{Connection, named_params};

use crate::db::ai::rag::{MAX_FOUND, passages, query_of};
use crate::db::database::Database;
use crate::db::error::DbError;
use crate::model::evaluation::product_document::ProductDocument;

/// The most passages of one document (a text of 20 million characters).
const MAX_PASSAGES: usize = 20_000;

/// The table and its index, made when they are missing (`database.rs`).
pub const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS product_documents (
        id      INTEGER PRIMARY KEY,
        name    TEXT NOT NULL,
        content BLOB,
        version TEXT
    );

    CREATE VIRTUAL TABLE IF NOT EXISTS product_document_passages USING fts5(
        text,
        document UNINDEXED,
        position UNINDEXED,
        tokenize = 'porter unicode61 remove_diacritics 2'
    );
";

/// Stores a new document (or the place of one) and gives its id. A new
/// object has no id yet (`id` 0): the database gives it.
///
/// Called by: nothing yet (the views rewritten on `db/`).
/// Calls: [`insert_product_document`].
pub fn create_product_document(db: &Database, document: &ProductDocument) -> Result<i64, DbError> {
    insert_product_document(db.connection(), document)
}

/// Writes a new document and gives its id, inside the transaction of its
/// caller (the copy of a plan writes its documents in its own).
///
/// Called by: [`create_product_document`], `evaluation_plan::create_evaluation`.
pub(in crate::db) fn insert_product_document(
    connection: &Connection,
    document: &ProductDocument,
) -> Result<i64, DbError> {
    ensure!(document.id == 0, DbError::Inconsistent("a new document has an id already".to_owned()));
    ensure!(!document.name.is_empty(), DbError::Inconsistent("a document without a name".to_owned()));

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

/// Indexes the text `text` of the document `id`, passage by passage with
/// their places. Run inside the transaction of its caller.
///
/// Called by: [`update_product_document`].
/// Calls: `rag::passages`.
fn write_passages(connection: &Connection, id: i64, text: &str) -> Result<(), DbError> {
    let cut = passages(text);

    ensure!(
        cut.len() <= MAX_PASSAGES,
        DbError::Inconsistent(format!("a document of more than {MAX_PASSAGES} passages"))
    );

    for (position, passage) in cut.iter().enumerate() {
        let position = i64::try_from(position).map_err(|error| DbError::Inconsistent(error.to_string()))?;

        connection.execute(
            "INSERT INTO product_document_passages (text, document, position) VALUES (:text, :id, :position)",
            named_params! { ":text": passage, ":id": id, ":position": position },
        )?;
    }

    Ok(())
}

/// Writes the document `document.id` anew: the file given in its reserved
/// place with its text `text` (read by the caller from the file), indexed
/// in the same transaction; or a correction of the place while it has no
/// file (`text` `None`). Refused once the file is given: the document is
/// then frozen.
///
/// Called by: `panels/logic/evaluation/inputs.rs` (`give_document`).
/// Calls: [`read_product_document`], [`write_passages`].
pub fn update_product_document(db: &Database, document: &ProductDocument, text: Option<&str>) -> Result<(), DbError> {
    ensure!(!document.name.is_empty(), DbError::Inconsistent("a document without a name".to_owned()));
    ensure!(
        document.content.is_some() == text.is_some(),
        DbError::Inconsistent("a file is given with its text, a place without one".to_owned())
    );

    let stored = read_product_document(db, document.id)?;

    ensure!(
        stored.content.is_none(),
        DbError::Inconsistent(format!("the document {} is given already: it is never changed", stored.name))
    );

    let transaction = db.connection().unchecked_transaction()?;
    let changed = transaction.execute(
        "UPDATE product_documents SET name = :name, content = :content, version = :version WHERE id = :id",
        named_params! {
            ":id": document.id,
            ":name": document.name,
            ":content": document.content,
            ":version": document.version,
        },
    )?;

    ensure!(changed == 1, DbError::NoRow { table: "product_documents", id: document.id });

    if let Some(text) = text {
        write_passages(&transaction, document.id, text)?;
    }

    transaction.commit()?;

    Ok(())
}

/// The text of the document `id` as its index keeps it: its passages in
/// their order, each on its own line (the spaces between the words of a
/// passage are one). Empty while the file is not given.
///
/// Called by: `panels/logic/evaluation/` (the question of a work unit, the
/// check of its quotes).
pub fn read_document_text(db: &Database, id: i64) -> Result<String, DbError> {
    let mut statement = db.connection().prepare(
        "SELECT text FROM product_document_passages WHERE document = :id ORDER BY CAST(position AS INTEGER)",
    )?;
    let mut rows = statement.query(named_params! { ":id": id })?;
    let mut text = String::new();

    for _ in 0..MAX_PASSAGES {
        let Some(row) = rows.next()? else { break };
        let passage: String = row.get("text")?;

        text.push_str(&passage);
        text.push('\n');
    }

    Ok(text)
}

/// The passages of the documents `documents` that fit `question` best (BM25
/// of FTS5), at most `count` and at most `MAX_FOUND`, the best first, each
/// with its document.
///
/// Called by: `panels/logic/evaluation/` (the question of a work unit whose
/// documents do not fit whole).
/// Calls: `rag::query_of`.
pub fn read_document_passages(
    db: &Database,
    documents: &[i64],
    question: &str,
    count: usize,
) -> Result<Vec<(i64, String)>, DbError> {
    let Some(query) = query_of(question) else {
        return Ok(Vec::new());
    };
    let wanted = count.min(MAX_FOUND);
    let mut ids = Vec::with_capacity(documents.len());

    for document in documents {
        ids.push(document.to_string());
    }

    let mut statement = db.connection().prepare(
        "SELECT text, document FROM product_document_passages
         WHERE product_document_passages MATCH :query
           AND document IN (SELECT value FROM json_each(:documents))
         ORDER BY bm25(product_document_passages) LIMIT :wanted",
    )?;
    let mut rows = statement.query(named_params! {
        ":query": query,
        ":documents": format!("[{}]", ids.join(",")),
        ":wanted": i64::try_from(wanted).map_err(|error| DbError::Inconsistent(error.to_string()))?,
    })?;
    let mut found = Vec::with_capacity(wanted);

    for _ in 0..wanted {
        let Some(row) = rows.next()? else { break };

        found.push((row.get("document")?, row.get("text")?));
    }

    Ok(found)
}

#[cfg(test)]
mod tests {
    //! Tests of the documents of the product: the place reserved, the file
    //! given, read back as it was and never changed again.

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

        assert!(matches!(update_product_document(&db, &given, None), Err(DbError::Inconsistent(_))));
        update_product_document(&db, &given, Some("The TOE is a router.\n\nIts TSF   filters packets.")).unwrap();

        assert_eq!(read_product_document(&db, id).unwrap(), given);
        assert!(matches!(
            update_product_document(
                &db,
                &ProductDocument { version: Some("1.4".to_owned()), ..given.clone() },
                Some("x")
            ),
            Err(DbError::Inconsistent(_))
        ));
        assert_eq!(read_product_document(&db, id).unwrap(), given);
        assert_eq!(read_document_text(&db, id).unwrap(), "The TOE is a router.\nIts TSF filters packets.\n");
        assert_eq!(read_document_passages(&db, &[id], "packet filter", 5).unwrap().len(), 1);
        assert!(read_document_passages(&db, &[id + 1], "packet", 5).unwrap().is_empty());
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
            update_product_document(&db, &ProductDocument { id: 3, ..place() }, None),
            Err(DbError::NoRow { .. })
        ));
    }
}
