//! The documents of standards an evaluator has loaded, kept in their
//! encrypted database (`db/user_db.rs`):
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
//! confidential, so they are kept only in the encrypted database.

use std::fmt;

use rusqlite::{OptionalExtension, Row, params};
use sha2::{Digest, Sha256};

use crate::db::db::DbError;
use crate::db::user_db::UserDb;

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
    /// A test project uses the document.
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
                formatter.write_str("a test project uses this document; remove it from the project first")
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

impl From<rusqlite::Error> for DocumentError {
    fn from(error: rusqlite::Error) -> DocumentError {
        DocumentError::Db(error.into())
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

/// The columns of a document, in the order of [`from_row`].
const COLUMNS: &str = "id, title, part, version, file_name, sha256, page_count";

fn from_row(row: &Row<'_>) -> rusqlite::Result<Document> {
    Ok(Document {
        id: row.get(0)?,
        fields: DocumentFields {
            title: row.get(1)?,
            part: row.get(2)?,
            version: row.get(3)?,
        },
        file_name: row.get(4)?,
        sha256: row.get(5)?,
        page_count: row.get(6)?,
    })
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

/// Every document, by title and part.
pub fn list(db: &UserDb) -> Result<Vec<Document>, DbError> {
    let mut statement = db.connection().prepare(&format!(
        "SELECT {COLUMNS} FROM standard_documents ORDER BY title COLLATE NOCASE, part COLLATE NOCASE, id"
    ))?;

    let documents = statement.query_map([], from_row)?.collect::<Result<Vec<_>, _>>()?;

    Ok(documents)
}

/// One document, or `None`.
pub fn get(db: &UserDb, id: i64) -> Result<Option<Document>, DbError> {
    Ok(db
        .connection()
        .query_row(&format!("SELECT {COLUMNS} FROM standard_documents WHERE id = ?1"), [id], from_row)
        .optional()?)
}

/// The document of a file, by its SHA-256 (lowercase hex).
pub fn by_sha256(db: &UserDb, sha256: &str) -> Result<Option<Document>, DbError> {
    Ok(db
        .connection()
        .query_row(&format!("SELECT {COLUMNS} FROM standard_documents WHERE sha256 = ?1"), [sha256], from_row)
        .optional()?)
}

/// Changes what the user told about a document.
pub fn update(db: &UserDb, id: i64, fields: &DocumentFields) -> Result<bool, DocumentError> {
    fields.check()?;

    Ok(db.connection().execute(
        "UPDATE standard_documents SET title = ?1, part = ?2, version = ?3 WHERE id = ?4",
        params![fields.title.trim(), fields.part.trim(), fields.version.trim(), id],
    )? > 0)
}

/// The text of a page (counted from 1), or `None` when there is no
/// such page.
pub fn page(db: &UserDb, id: i64, number: u32) -> Result<Option<String>, DbError> {
    Ok(db
        .connection()
        .query_row(
            "SELECT text FROM standard_pages WHERE document_id = ?1 AND number = ?2",
            params![id, number],
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

/// Searches the text of the documents (all, or one), in the form of
/// `quote.rs` (spaces, divided words, case do not matter). At most
/// [`MAX_HITS`] hits, in the order of the documents and pages.
pub fn search(db: &UserDb, text: &str, document: Option<i64>) -> Result<Vec<Hit>, DbError> {
    let wanted = normalize(text);

    if wanted.is_empty() {
        return Ok(Vec::new());
    }

    let mut statement = db.connection().prepare(
        "SELECT document_id, number, text FROM standard_pages
         WHERE ?1 IS NULL OR document_id = ?1 ORDER BY document_id, number",
    )?;

    let mut rows = statement.query([document])?;
    let mut hits = Vec::new();

    while let Some(row) = rows.next()? {
        let page: String = row.get(2)?;
        let (normal, places) = normalize_with_places(&page);

        let mut from = 0;

        while let Some(found) = normal[from..].find(&wanted) {
            let at = from + found;

            hits.push(Hit {
                document_id: row.get(0)?,
                page: row.get(1)?,
                snippet: snippet(&page, places[at], places[at + wanted.len() - 1]),
            });

            if hits.len() >= MAX_HITS {
                return Ok(hits);
            }

            from = at + wanted.len();
        }
    }

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

/// Removes a document; refused while a test project uses it.
pub fn delete(db: &UserDb, id: i64) -> Result<bool, DocumentError> {
    let used: bool = db.connection().query_row(
        "SELECT EXISTS (SELECT 1 FROM project_documents WHERE document_id = ?1)",
        [id],
        |row| row.get(0),
    )?;

    if used {
        return Err(DocumentError::InUse);
    }

    let transaction = db.connection().unchecked_transaction()?;

    // Licensed text is overwritten in the file, not only marked free.
    transaction.execute_batch("PRAGMA secure_delete = ON")?;
    transaction.execute("DELETE FROM standard_pages WHERE document_id = ?1", [id])?;
    let deleted = transaction.execute("DELETE FROM standard_documents WHERE id = ?1", [id])? > 0;
    transaction.commit()?;

    Ok(deleted)
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "documents_tests.rs"]
mod tests;
