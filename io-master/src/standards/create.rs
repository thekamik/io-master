//! Loading a document of a standard: its file and the text of its
//! pages, refused when the same file is loaded already.

use crate::db::standards::create;
use crate::db::user::database::UserDb;

use super::model::{DocumentError, DocumentFields, sha256_hex};
use super::read::by_sha256;

/// Stores a document: the file and the text of its pages (one text
/// per page, in order). Refused when the same file is loaded already.
pub fn add(db: &UserDb, fields: &DocumentFields, file_name: &str, pdf: &[u8], pages: &[String]) -> Result<i64, DocumentError> {
    fields.check()?;

    if pages.is_empty() {
        return Err(DocumentError::Invalid("the document has no pages".to_owned()));
    }

    let sha256 = sha256_hex(pdf);

    if let Some(existing) = by_sha256(db, &sha256)? {
        return Err(DocumentError::Duplicate(existing.id));
    }

    Ok(create::document(db, fields, file_name, &sha256, pdf, pages)?)
}


// Unit tests, in their own file to keep this one readable. They
// cover the whole folder (create, read, update, delete).
#[cfg(test)]
#[path = "create_tests.rs"]
mod tests;
