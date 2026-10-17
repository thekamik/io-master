//! The documents of standards an evaluator has loaded, kept in their
//! encrypted database (`db/user/`):
//!
//! ```text
//! standard_documents   title, part, version, the file name, the
//!                      SHA-256 of the file, the number of pages, the
//!                      PDF itself
//! standard_pages       the text of every page (`pdf_text.rs`)
//! ```
//!
//! A standard often comes in several documents (CC Part 1 to 5 and
//! the CEM); each is loaded on its own, and a test project names the
//! ones it uses (`projects.rs`). The same file is loaded once: its
//! SHA-256 finds it again, also when a project is imported (step 22 of
//! `docs/todo.md`).
//!
//! The files are licensed copies and the work on them is
//! confidential, so they are kept only in the encrypted database. The
//! SQL is in `db/standards/`.

use std::fmt;

use sha2::{Digest, Sha256};

use crate::db::error::DbError;
use crate::db::standards::{create, delete as remove, read, update as change};
use crate::db::user::database::UserDb;

use super::quote::{normalize, normalize_with_places};
use super::scheme::MAX_SHORT;

/// The most hits a search gives.
pub const MAX_HITS: usize = 200;

/// The characters of text around a hit of a search, on each side.
const SNIPPET: usize = 80;

/// What the user tells about a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentFields {
    /// "Common Criteria for Information Technology Security Evaluation".
    pub title: String,
    /// "Part 3: Security assurance components"; may be empty.
    pub part: String,
    /// "CC:2022 Revision 1"; may be empty.
    pub version: String,
}

/// A loaded document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub id: i64,
    pub fields: DocumentFields,
    /// The name of the file it was loaded from.
    pub file_name: String,
    /// The SHA-256 of the file, in lowercase hex.
    pub sha256: String,
    pub page_count: u32,
}

/// A hit of a search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub document_id: i64,
    pub page: u32,
    /// The text around the hit, on one line.
    pub snippet: String,
}

/// Why a document was not stored or removed.
#[derive(Debug)]
pub enum DocumentError {
    /// A field breaks a rule; holds the reason.
    Invalid(String),
    /// The same file is loaded already, as this document.
    Duplicate(i64),
    /// A test project or an evaluation project uses the document.
    InUse,
    /// The database failed.
    Db(DbError),
}

impl fmt::Display for DocumentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DocumentError::Invalid(reason) => formatter.write_str(reason),
            DocumentError::Duplicate(_) => formatter.write_str("this file is loaded already"),
            DocumentError::InUse => {
                formatter.write_str(
                    "a test project or an evaluation uses this document; remove it from the test project, \
                     or delete the evaluation, first",
                )
            },
            DocumentError::Db(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for DocumentError {}

impl From<DbError> for DocumentError {
    fn from(error: DbError) -> DocumentError {
        DocumentError::Db(error)
    }
}

/// The SHA-256 of a file, in lowercase hex.
pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
}

impl DocumentFields {
    fn check(&self) -> Result<(), DocumentError> {
        if self.title.trim().is_empty() {
            return Err(DocumentError::Invalid("the document needs a title".to_owned()));
        }

        for (name, text) in [("title", &self.title), ("part", &self.part), ("version", &self.version)] {
            if text.chars().count() > MAX_SHORT {
                return Err(DocumentError::Invalid(format!("the {name} is longer than {MAX_SHORT} characters")));
            }

            if text.chars().any(char::is_control) {
                return Err(DocumentError::Invalid(format!("the {name} holds a control character")));
            }
        }

        Ok(())
    }
}

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

/// Every document, by title and part.
pub fn list(db: &UserDb) -> Result<Vec<Document>, DbError> {
    read::documents(db)
}

/// One document, or `None`.
pub fn get(db: &UserDb, id: i64) -> Result<Option<Document>, DbError> {
    read::document(db, id)
}

/// The document of a file, by its SHA-256 (lowercase hex).
pub fn by_sha256(db: &UserDb, sha256: &str) -> Result<Option<Document>, DbError> {
    read::by_sha256(db, sha256)
}

/// Changes what the user told about a document.
pub fn update(db: &UserDb, id: i64, fields: &DocumentFields) -> Result<bool, DocumentError> {
    fields.check()?;

    Ok(change::document(db, id, fields)?)
}

/// The text of a page (counted from 1), or `None` when there is no
/// such page.
pub fn page(db: &UserDb, id: i64, number: u32) -> Result<Option<String>, DbError> {
    read::page(db, id, number)
}

/// The PDF of a document, or `None`.
pub fn pdf(db: &UserDb, id: i64) -> Result<Option<Vec<u8>>, DbError> {
    read::pdf(db, id)
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

    read::each_page(db, document, |document_id, number, page| {
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

/// Removes a document; refused while a test project or an
/// evaluation project uses it.
pub fn delete(db: &UserDb, id: i64) -> Result<bool, DocumentError> {
    if read::in_use(db, id)? {
        return Err(DocumentError::InUse);
    }

    // The licensed text is overwritten in the file (db/standards/delete.rs).
    Ok(remove::document(db, id)?)
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "documents_tests.rs"]
mod tests;
