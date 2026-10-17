//! Storing a document of a standard: the file and the text of its
//! pages. The rules (title, a file loaded once) are checked by
//! `evaluation/documents.rs`.

use rusqlite::params;

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::evaluation::documents::DocumentFields;

/// Stores a document with the text of its pages (one text per page,
/// in order, numbered from 1), in one transaction; returns its number.
pub fn document(
    db: &UserDb,
    fields: &DocumentFields,
    file_name: &str,
    sha256: &str,
    pdf: &[u8],
    pages: &[String],
) -> Result<i64, DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute(
        "INSERT INTO standard_documents (title, part, version, file_name, sha256, page_count, pdf)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            fields.title.trim(),
            fields.part.trim(),
            fields.version.trim(),
            file_name,
            sha256,
            pages.len() as i64,
            pdf
        ],
    )?;

    let id = transaction.last_insert_rowid();

    {
        let mut insert =
            transaction.prepare("INSERT INTO standard_pages (document_id, number, text) VALUES (?1, ?2, ?3)")?;

        for (index, text) in pages.iter().enumerate() {
            insert.execute(params![id, index as i64 + 1, text])?;
        }
    }

    transaction.commit()?;

    Ok(id)
}
