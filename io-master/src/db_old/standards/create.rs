//! Storing a document of a standard: the file and the text of its
//! pages. The rules (title, a file loaded once) are checked by
//! `views/converter/standard_documents/stored.rs`.

use rusqlite::named_params;

use crate::db_old::error::DbError;
use crate::db_old::user::database::UserDb;
use crate::model_old::standards::DocumentFields;

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
         VALUES (:title, :part, :version, :file_name, :sha256, :page_count, :pdf)",
        named_params! { ":title": fields.title, ":part": fields.part, ":version": fields.version, ":file_name": file_name, ":sha256": sha256, ":page_count": pages.len() as i64, ":pdf": pdf },
    )?;

    let id = transaction.last_insert_rowid();

    {
        let mut insert = transaction
            .prepare("INSERT INTO standard_pages (document_id, number, text) VALUES (:document_id, :number, :text)")?;

        for (index, text) in pages.iter().enumerate() {
            insert.execute(named_params! { ":document_id": id, ":number": index as i64 + 1, ":text": text })?;
        }
    }

    transaction.commit()?;

    Ok(id)
}
