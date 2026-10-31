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
//! passages, as the knowledge is (`db/models/ai/rag.rs`), and indexed by FTS5 with
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

use crate::db::error::DbError;
use crate::db::models::ai::rag::{MAX_FOUND, passages, query_of};
use crate::db::storage::user_database::UserDatabase;
use crate::model::evaluation::product_document::ProductDocument;

/// The most passages of one document (a text of 20 million characters).
const MAX_PASSAGES: usize = 20_000;

/// The table and its index, made when they are missing (`schema.rs`, `USER_TABLES`).
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

/// Stores a new document of the product, or the place of one in a plan (its
/// name only: "ST"), and gives its id.
///
/// Input: `db`, the open database of the user; `document`, the new document,
/// its `id` 0 (the database gives it).
///
/// Output: the id of the document (3); an error as
/// [`insert_product_document`].
///
/// Called by: `panels/logic/evaluation_plan/plans.rs`: `add_document_place`.
/// Calls: [`insert_product_document`].
pub fn create_product_document(db: &UserDatabase, document: &ProductDocument) -> Result<i64, DbError> {
    insert_product_document(db.connection(), document)
}

/// Writes a new document and gives its id, inside the transaction of its
/// caller, so that the copy of a plan writes its documents in its own.
///
/// Input: `connection`, the open database; `document`, the new document, its
/// `id` 0 and a name.
///
/// Output: the id of the document; an error when it has an id or no name.
///
/// Called by: [`create_product_document`]; `evaluation_plan.rs`:
/// `copied_places`.
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

/// Reads the document `id`, with its file once given.
///
/// Input: `db`, the open database of the user; `id`, the id of the document
/// (3).
///
/// Output: the document (its file `None` while the place is only reserved);
/// `DbError::NoRow` when there is none.
///
/// Called by: [`update_product_document`]; `evaluation_plan.rs`:
/// `copied_places`; `panels/logic/evaluation/answer.rs`: `checked_quotes`;
/// `panels/logic/evaluation/inputs.rs`: `give_document`, `inputs`;
/// `panels/logic/evaluation/question.rs`: `documents_text`, `unit_json`;
/// `panels/logic/evaluation/results.rs`: `check_quotes`;
/// `panels/logic/evaluation/run.rs`: `missing_inputs`;
/// `panels/logic/evaluation_plan/fill.rs`: `names_of`;
/// `panels/views/evaluation/work_units.rs`: `needs_of`;
/// `panels/views/evaluation_plan/plans.rs`: `plan_view`.
pub fn read_product_document(db: &UserDatabase, id: i64) -> Result<ProductDocument, DbError> {
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

/// Indexes the text of a document passage by passage, each with its place.
/// Run inside the transaction of its caller.
///
/// Input: `connection`, the open database inside a transaction; `id`, the id
/// of the document; `text`, its text read from its file.
///
/// Output: nothing; an error when the text makes more than `MAX_PASSAGES`
/// passages.
///
/// Called by: [`update_product_document`].
/// Calls: `models/ai/rag.rs`: `passages`.
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
/// place with its text, indexed in the same transaction; or a correction of
/// the place while it has no file. Refused once the file is given: the
/// document is then frozen.
///
/// Input:
/// - `db`: the open database of the user.
/// - `document`: the document as it is to be kept, its `id` that of a
///   stored one, a name; `content` the file, or `None` for a correction of
///   the place.
/// - `text`: the text of the file, read by the caller (a PDF through
///   Poppler); given exactly when the file is.
///
/// Output: nothing; an error when it has no name, when the file and the text
/// do not come together, when the file is given already, or
/// `DbError::NoRow` when there is no document `document.id`; nothing is
/// then written.
///
/// Called by: `panels/logic/evaluation/inputs.rs`: `give_document`.
/// Calls: [`read_product_document`], [`write_passages`].
pub fn update_product_document(
    db: &UserDatabase,
    document: &ProductDocument,
    text: Option<&str>,
) -> Result<(), DbError> {
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

/// Reads the text of a document as its index keeps it: its passages in
/// their order, each on its own line (the spaces between the words of a
/// passage are one), without reading the file again.
///
/// Input: `db`, the open database of the user; `id`, the id of the document.
///
/// Output: the text; empty while the file is not given.
///
/// Called by: `panels/logic/evaluation/answer.rs`: `checked_quotes`;
/// `panels/logic/evaluation/question.rs`: `documents_text`.
pub fn read_document_text(db: &UserDatabase, id: i64) -> Result<String, DbError> {
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

/// Finds the passages of the documents of a work unit that fit a question
/// best (BM25 of FTS5), the best first, when the whole documents do not fit
/// into the context of the model.
///
/// Input:
/// - `db`: the open database of the user.
/// - `documents`: the ids of the documents to search.
/// - `question`: the text to look for (the requirement of the work unit).
/// - `count`: the most passages wanted.
///
/// Output: at most `count` and at most `MAX_FOUND` passages, each with its
/// document (`[(3, "The TOE protects ...")]`); none when the question has no
/// word to look for.
///
/// Called by: `panels/logic/evaluation/question.rs`: `documents_text`.
/// Calls: `models/ai/rag.rs`: `query_of`.
pub fn read_document_passages(
    db: &UserDatabase,
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
