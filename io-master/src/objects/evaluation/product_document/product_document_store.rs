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
//! passages, as the knowledge is (`objects/ai/rag/rag_store.rs`), and indexed by FTS5 with
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

use crate::infrastructure::storage::error::DbError;
use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::ai::rag::rag_store::{MAX_FOUND, PASSAGE_CHARS, STOP_WORDS, passages, query_of};
use crate::objects::evaluation::product_document::product_document_model::ProductDocument;

/// The most passages of one document (a text of 20 million characters).
const MAX_PASSAGES: usize = 20_000;

/// The table and its index, made when they are missing (`objects/schema.rs`, `USER_TABLES`).
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
/// Called by: `objects/evaluation/evaluation_plan/evaluation_plan_rules/planning.rs`: `add_document_place`.
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
/// Called by: [`create_product_document`]; `objects/evaluation/evaluation_plan/evaluation_plan_store.rs`:
/// `copied_places`.
pub(in crate::objects::evaluation) fn insert_product_document(
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
/// Called by: `objects/evaluation/evaluation_plan/evaluation_plan_store.rs`: `copied_places`;
/// `objects/evaluation/work_unit/work_unit_ai/proposal.rs`: `names_of`;
/// `panels/views/evaluation_plan/plans.rs`: `plan_view` (the places of a plan, which hold no
/// file). Where a document of an evaluation is read for its name, its version or whether it is
/// given, [`read_document_record`] reads it without its file.
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

/// A document without its file, and the size of that file in bytes: `None`
/// while the place is only reserved.
pub type DocumentRecord = (ProductDocument, Option<u64>);

/// Reads the document `id` without its file (user, 2026-11-10): its name,
/// its version, and the size of its file, which tells whether it is given.
/// The file of a document may hold 256 MiB, and most readers ask only for
/// its name: the question of each work unit, a work unit shown, a report.
///
/// Input: `db`, the open database of the user; `id`, the id of the document
/// (3).
///
/// Output: the document, its `content` always `None` (the file is not
/// read), and the size of its file (`(ProductDocument { name: "ST", .. },
/// Some(5242880))`), `None` while the place is only reserved;
/// `DbError::NoRow` when there is none.
///
/// Called by: [`update_product_document`];
/// `objects/evaluation/product_document/product_document_rules.rs`: `give_document`;
/// `objects/evaluation/evaluation_plan/evaluation_plan_ai.rs`: `inputs_of`;
/// `objects/evaluation/work_unit/work_unit_ai/assessment_question.rs`: `documents_text`, `unit_json`;
/// `objects/evaluation/work_unit/work_unit_rules/results.rs`: `quote_sources`;
/// `objects/evaluation/report_template/report_template_rules/filling.rs`: `lists_of`;
/// `panels/logic/evaluation/inputs.rs`: `inputs`; `panels/views/evaluation/work_units.rs`: `needs_of`.
pub fn read_document_record(db: &UserDatabase, id: i64) -> Result<DocumentRecord, DbError> {
    let connection = db.connection();
    // `length` of a file is told by the head of its row: SQLite does not
    // read the file for it.
    let mut statement = connection
        .prepare_cached("SELECT id, name, version, length(content) AS bytes FROM product_documents WHERE id = :id")?;
    let mut rows = statement.query(named_params! { ":id": id })?;
    let Some(row) = rows.next()? else {
        return Err(DbError::NoRow { table: "product_documents", id });
    };
    let bytes = match row.get::<_, Option<i64>>("bytes")? {
        Some(bytes) => Some(u64::try_from(bytes).map_err(|_| DbError::Corrupt {
            table: "product_documents",
            column: "content",
            value: bytes.to_string(),
        })?),
        None => None,
    };

    Ok((
        ProductDocument { id: row.get("id")?, name: row.get("name")?, content: None, version: row.get("version")? },
        bytes,
    ))
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
/// Calls: `objects/ai/rag/rag_store.rs`: `passages`.
fn write_passages(connection: &Connection, id: i64, text: &str) -> Result<(), DbError> {
    let cut = passages(text, PASSAGE_CHARS);

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
/// Called by: `objects/evaluation/product_document/product_document_rules.rs`: `give_document`.
/// Calls: [`read_document_record`], [`write_passages`].
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

    let (stored, bytes) = read_document_record(db, document.id)?;

    ensure!(
        bytes.is_none(),
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

/// Refuses a document whose file is given but whose text has no passages:
/// the program writes the file and its passages in one transaction, so such
/// a document comes from a database the program did not write (an earlier
/// layout of the tables, a file changed outside the program). Without this
/// check its text would read as empty, and the AI would assess an empty
/// document.
///
/// Input: `connection`, the open database; `id`, the id of a document.
/// Output: nothing for a place not given yet or a document with passages;
/// `DbError::NoRow` for a document missing, `DbError::Corrupt` for a file
/// without passages.
///
/// Called by: [`read_document_text`], [`read_document_passages`].
fn check_indexed(connection: &Connection, id: i64) -> Result<(), DbError> {
    let mut statement = connection.prepare(
        "SELECT content IS NOT NULL AS given,
                (SELECT count(*) FROM product_document_passages WHERE document = :id) AS passages
         FROM product_documents WHERE id = :id",
    )?;
    let mut rows = statement.query(named_params! { ":id": id })?;
    let Some(row) = rows.next()? else {
        return Err(DbError::NoRow { table: "product_documents", id });
    };
    let given: bool = row.get("given")?;
    let passages: i64 = row.get("passages")?;

    ensure!(
        !given || passages > 0,
        DbError::Corrupt {
            table: "product_document_passages",
            column: "text",
            value: format!("none for the given document {id}"),
        }
    );

    Ok(())
}

/// Reads the text of a document as its index keeps it: its passages in
/// their order, each on its own line (the spaces between the words of a
/// passage are one), without reading the file again.
///
/// Input: `db`, the open database of the user; `id`, the id of the document.
///
/// Output: the text; empty while the file is not given; the errors of
/// [`check_indexed`] for a document missing or a file without passages.
///
/// Called by: `objects/evaluation/work_unit/work_unit_rules/results.rs`: `quote_sources`;
/// `objects/evaluation/work_unit/work_unit_ai/assessment_question.rs`: `documents_text`.
/// Calls: [`check_indexed`].
pub fn read_document_text(db: &UserDatabase, id: i64) -> Result<String, DbError> {
    check_indexed(db.connection(), id)?;

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
/// word to look for; the errors of [`check_indexed`] for a document missing
/// or a file without passages.
///
/// Called by: `objects/evaluation/work_unit/work_unit_ai/assessment_question.rs`: `documents_text`.
/// Calls: [`check_indexed`]; `objects/ai/rag/rag_store.rs`: `query_of`.
pub fn read_document_passages(
    db: &UserDatabase,
    documents: &[i64],
    question: &str,
    count: usize,
) -> Result<Vec<(i64, String)>, DbError> {
    // Every document is checked first, so that a file without passages is
    // an error, never a search that finds nothing in it.
    for document in documents {
        check_indexed(db.connection(), *document)?;
    }

    let Some(query) = query_of(question, &STOP_WORDS) else {
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
    //! given, read back as it was (and without its file) and never changed
    //! again.

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
        assert_eq!(read_document_record(&db, id).unwrap(), (ProductDocument { id, ..place() }, None));

        let given = ProductDocument {
            id,
            content: Some(vec![0x25, 0x50, 0x44, 0x46, 0]),
            version: Some("1.3".to_owned()),
            ..place()
        };

        assert!(matches!(update_product_document(&db, &given, None), Err(DbError::Inconsistent(_))));
        update_product_document(&db, &given, Some("The TOE is a router.\n\nIts TSF   filters packets.")).unwrap();

        assert_eq!(read_product_document(&db, id).unwrap(), given);
        // Without its file: the same document, and the size of the file.
        assert_eq!(
            read_document_record(&db, id).unwrap(),
            (ProductDocument { content: None, ..given.clone() }, Some(5))
        );
        assert!(matches!(read_document_record(&db, id + 1), Err(DbError::NoRow { .. })));
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
        assert!(matches!(read_document_passages(&db, &[id + 1], "packet", 5), Err(DbError::NoRow { .. })));
    }

    #[test]
    fn a_given_file_without_passages_is_refused_never_read_as_empty() {
        let (_directory, db) = database();
        let id = create_product_document(&db, &place()).unwrap();

        // A place not given yet has no text, and that is no error.
        assert_eq!(read_document_text(&db, id).unwrap(), "");

        // A file written without its passages, as an earlier layout of the
        // tables left it: the program itself never writes this.
        db.connection()
            .execute(
                "UPDATE product_documents SET content = :content WHERE id = :id",
                named_params! { ":content": vec![0x25_u8, 0x50, 0x44, 0x46], ":id": id },
            )
            .unwrap();

        assert!(matches!(read_document_text(&db, id), Err(DbError::Corrupt { .. })));
        assert!(matches!(read_document_passages(&db, &[id], "router", 5), Err(DbError::Corrupt { .. })));
        assert!(matches!(read_document_text(&db, id + 1), Err(DbError::NoRow { .. })));
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
