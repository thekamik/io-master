//! The documents of standards an evaluator has loaded (steps 16 to 22
//! of `docs/todo.md`): their types and rules. Kept lastingly in the
//! encrypted database of the evaluator (`db/standards/`).
//!
//! ```text
//! model.rs      the types, the rules of the fields
//! create.rs     load a document: its file and the text of its pages
//! read.rs       the list, one document, a page, the PDF, the search
//! update.rs     what the user told about a document
//! delete.rs     remove a document no project uses
//! pdf_text.rs   the text of a PDF page by page (Poppler)
//! quote.rs      one normal form of text; is a quote on its page
//! ```
//!
//! A standard often comes in several documents (CC Part 1 to 5 and
//! the CEM); each is loaded on its own, and a test project names the
//! ones it uses. The same file is loaded once: its SHA-256 finds it
//! again, also when a project is imported. A document is only loaded
//! and read, never edited as an object in memory: its functions go to
//! the database directly.
//!
//! The files are licensed copies and the work on them is
//! confidential, so they are kept only in the encrypted database.

use std::fmt;

use sha2::{Digest, Sha256};

use crate::db::error::DbError;

/// The longest title, part or version, in characters.
pub const MAX_SHORT: usize = 200;

/// The most hits a search gives.
pub const MAX_HITS: usize = 200;

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
    /// Checks what the user told about a document.
    pub fn check(&self) -> Result<(), DocumentError> {
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
