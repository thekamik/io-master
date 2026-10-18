//! The documents of standards as a resource (steps 16 to 22 of
//! `docs/todo.md`, operations named in step 40). A document is only
//! loaded and read, never edited as an object in memory: its operations
//! go to the database directly (`db/standards/`).
//!
//! ```text
//! list()        GET     /documents
//! get()         GET     /documents/{id}
//! by_sha256()   GET     /documents?sha256=...     the document of a file
//! page()        GET     /documents/{id}/pages/{n}
//! pdf()         GET     /documents/{id}/pdf
//! search()      GET     /documents/search?text=...
//! create()      POST    /documents                a file loaded once (its SHA-256)
//! update()      PUT     /documents/{id}           what the user told about it
//! delete()      DELETE  /documents/{id}           refused while a project uses it
//! ```
//!
//! The loading of PDFs, a long work, is `load.rs`.

use super::quote::{normalize, normalize_with_places};
use crate::business::standards::error::DocumentError;
use crate::db::error::DbError;
use crate::db::evaluation_support as db_evaluation_support;
use crate::db::standards as db_standards;
use crate::db::user::database::UserDb;
use crate::model::standards::{Document, DocumentFields, Hit, MAX_HITS, sha256_hex};

/* ---------------------------------------------------------------
 * Read
 * ------------------------------------------------------------- */

/// The characters of text around a hit of a search, on each side.
const SNIPPET: usize = 80;

/// Every document, by title and part.
pub fn list(db: &UserDb) -> Result<Vec<Document>, DbError> {
    db_standards::read::documents(db)
}

/// One document, or `None`.
pub fn get(db: &UserDb, id: i64) -> Result<Option<Document>, DbError> {
    db_standards::read::document(db, id)
}

/// The document of a file, by its SHA-256 (lowercase hex).
pub fn by_sha256(db: &UserDb, sha256: &str) -> Result<Option<Document>, DbError> {
    db_standards::read::by_sha256(db, sha256)
}

/// The text of a page (counted from 1), or `None` when there is no
/// such page.
pub fn page(db: &UserDb, id: i64, number: u32) -> Result<Option<String>, DbError> {
    db_standards::read::page(db, id, number)
}

/// The PDF of a document, or `None`.
pub fn pdf(db: &UserDb, id: i64) -> Result<Option<Vec<u8>>, DbError> {
    db_standards::read::pdf(db, id)
}

/// Searches the text of the documents (all, or one), in the form of
/// `quote.rs` (spaces, divided words, case do not matter). At most
/// [`MAX_HITS`] hits, in the order of the documents and pages.
pub fn search(db: &UserDb, text: &str, document: Option<i64>) -> Result<Vec<Hit>, DbError> {
    let wanted = normalize(text);

    if wanted.is_empty() {
        return Ok(Vec::new());
    }

    let mut hits = Vec::new();

    db_standards::read::each_page(db, document, |document_id, number, page| {
        let (normal, places) = normalize_with_places(page);

        let mut from = 0;

        while let Some(found) = normal[from..].find(&wanted) {
            let at = from + found;

            hits.push(Hit {
                document_id,
                page: number,
                snippet: snippet(page, places[at], places[at + wanted.len() - 1]),
            });

            if hits.len() >= MAX_HITS {
                return false;
            }

            from = at + wanted.len();
        }

        true
    })?;

    Ok(hits)
}

/// The text of the page around a hit that runs from byte `first` to
/// the character starting at byte `last`, as it is in the page, on
/// one line.
fn snippet(page: &str, first: usize, last: usize) -> String {
    let end = last + page[last..].chars().next().map_or(0, char::len_utf8);
    let before: String = page[..first].chars().rev().take(SNIPPET).collect::<Vec<_>>().into_iter().rev().collect();
    let after: String = page[end..].chars().take(SNIPPET).collect();
    let open = if before.len() < first { "…" } else { "" };
    let close = if end + after.len() < page.len() { "…" } else { "" };
    let text = format!("{open}{before}{}{after}{close}", &page[first..end]);

    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/* ---------------------------------------------------------------
 * Create
 * ------------------------------------------------------------- */

/// Stores a document: the file and the text of its pages (one text
/// per page, in order). Refused when the same file is loaded already.
pub fn create(
    db: &UserDb,
    fields: &DocumentFields,
    file_name: &str,
    pdf: &[u8],
    pages: &[String],
) -> Result<i64, DocumentError> {
    // The normal form (model/): trimmed here, stored as it is.
    let fields = &fields.normalized();
    fields.check()?;

    if pages.is_empty() {
        return Err(DocumentError::Invalid("the document has no pages".to_owned()));
    }

    let sha256 = sha256_hex(pdf);

    if let Some(existing) = by_sha256(db, &sha256)? {
        return Err(DocumentError::Duplicate(existing.id));
    }

    Ok(db_standards::create::document(db, fields, file_name, &sha256, pdf, pages)?)
}

/* ---------------------------------------------------------------
 * Update
 * ------------------------------------------------------------- */

/// Changes what the user told about a document.
pub fn update(db: &UserDb, id: i64, fields: &DocumentFields) -> Result<bool, DocumentError> {
    // The normal form (model/): trimmed here, stored as it is.
    let fields = &fields.normalized();
    fields.check()?;

    Ok(db_standards::update::document(db, id, fields)?)
}

/* ---------------------------------------------------------------
 * Delete
 * ------------------------------------------------------------- */

/// Removes a document; refused while a quote of an OSCAL document names
/// it (the evaluation support).
pub fn delete(db: &UserDb, id: i64) -> Result<bool, DocumentError> {
    // A document a quote of the evaluation support names stays.
    if db_evaluation_support::read::quotes_source(db, id)? {
        return Err(DocumentError::InUse);
    }

    // The licensed text is overwritten in the file (db/standards/delete.rs).
    Ok(db_standards::delete::document(db, id)?)
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "documents_tests.rs"]
mod tests;
