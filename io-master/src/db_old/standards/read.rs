//! Reading the documents of standards, their pages and PDFs.

use rusqlite::{OptionalExtension, named_params};

use crate::db_old::error::DbError;
use crate::db_old::user::database::UserDb;
use crate::model_old::standards::Document;

use super::rows::{COLUMNS, document_from_row};

/// Every document, by title and part.
pub fn documents(db: &UserDb) -> Result<Vec<Document>, DbError> {
    let mut statement = db.connection().prepare(&format!(
        "SELECT {COLUMNS} FROM standard_documents ORDER BY title COLLATE NOCASE, part COLLATE NOCASE, id"
    ))?;

    Ok(statement.query_map([], document_from_row)?.collect::<Result<Vec<_>, _>>()?)
}

/// One document, or `None`.
pub fn document(db: &UserDb, id: i64) -> Result<Option<Document>, DbError> {
    Ok(db
        .connection()
        .query_row(&format!("SELECT {COLUMNS} FROM standard_documents WHERE id = ?1"), [id], document_from_row)
        .optional()?)
}

/// The document of a file, by its SHA-256 (lowercase hex).
pub fn by_sha256(db: &UserDb, sha256: &str) -> Result<Option<Document>, DbError> {
    Ok(db
        .connection()
        .query_row(&format!("SELECT {COLUMNS} FROM standard_documents WHERE sha256 = ?1"), [sha256], document_from_row)
        .optional()?)
}

/// The text of a page (counted from 1), or `None` when there is no
/// such page.
pub fn page(db: &UserDb, id: i64, number: u32) -> Result<Option<String>, DbError> {
    Ok(db
        .connection()
        .query_row(
            "SELECT text FROM standard_pages WHERE document_id = :document_id AND number = :number",
            named_params! { ":document_id": id, ":number": number },
            |row| row.get(0),
        )
        .optional()?)
}

/// The PDF of a document, or `None`.
pub fn pdf(db: &UserDb, id: i64) -> Result<Option<Vec<u8>>, DbError> {
    Ok(db
        .connection()
        .query_row("SELECT pdf FROM standard_documents WHERE id = ?1", [id], |row| row.get(0))
        .optional()?)
}

/// Calls `visit` with the document, the number and the text of every
/// page (of all documents, or of one), in the order of the documents
/// and pages, one page at a time; stops when `visit` returns `false`.
pub fn each_page(
    db: &UserDb,
    document: Option<i64>,
    mut visit: impl FnMut(i64, u32, &str) -> bool,
) -> Result<(), DbError> {
    let mut statement = db.connection().prepare(
        "SELECT document_id, number, text FROM standard_pages
         WHERE ?1 IS NULL OR document_id = ?1 ORDER BY document_id, number",
    )?;

    let mut rows = statement.query([document])?;

    while let Some(row) = rows.next()? {
        let text: String = row.get(2)?;

        if !visit(row.get(0)?, row.get(1)?, &text) {
            break;
        }
    }

    Ok(())
}
