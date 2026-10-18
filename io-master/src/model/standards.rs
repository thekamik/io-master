//! The documents of standards an evaluator has loaded (steps 16 to 22
//! of `docs/todo.md`): their types and rules. Kept lastingly in the
//! encrypted database of the evaluator (`db/standards/`).
//!
//! ```text
//! model/standards.rs                the types, the rules of the fields (this file)
//! business/standards/documents.rs   the documents as a resource: load, list, page,
//!                                   PDF, search, update, delete
//! business/standards/load.rs        the loading of PDFs, a long work
//! business/standards/pdf_text.rs    the text of a PDF page by page (Poppler)
//! business/standards/quote.rs       one normal form of text; is a quote on its page
//! ```
//!
//! A standard often comes in several documents (CC Part 1 to 5 and
//! the CEM); each is loaded on its own, and the OSCAL documents of step 41 name the
//! ones they come from (resources of their back matter, by the SHA-256
//! of the file). The same file is loaded once: its SHA-256 finds it
//! again. A document is only loaded
//! and read, never edited as an object in memory: its functions go to
//! the database directly.
//!
//! The files are licensed copies and the work on them is
//! confidential, so they are kept only in the encrypted database.

use sha2::{Digest, Sha256};

use crate::model::text::{Field, Invalid, check_text};

/// The longest title, part or version, in characters.
pub const MAX_SHORT: usize = 200;

/// The most hits a search gives.
pub const MAX_HITS: usize = 200;

/// The longest quote that is accepted, in characters. A source is a
/// passage, not a page (`business/standards/quote.rs`).
pub const MAX_QUOTE: usize = 2000;

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

/// The SHA-256 of a file, in lowercase hex.
pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
}

impl DocumentFields {
    /// Checks what the user told about a document.
    pub fn check(&self) -> Result<(), Invalid> {
        if self.title.trim().is_empty() {
            return Err(Invalid::new(Field::Title, "the document needs a title"));
        }

        check_text(Field::Title, &self.title, MAX_SHORT, false)?;
        check_text(Field::Part, &self.part, MAX_SHORT, false)?;
        check_text(Field::Version, &self.version, MAX_SHORT, false)
    }
}

/// The normal form of what is stored: the texts of one line without
/// spaces around them. **The only place where text is trimmed** (step
/// 40): the work normalizes what it gets before it checks and stores
/// it, `db/` writes it as it is, so memory and the database hold the
/// same text.
impl DocumentFields {
    pub fn normalized(&self) -> DocumentFields {
        DocumentFields {
            title: self.title.trim().to_owned(),
            part: self.part.trim().to_owned(),
            version: self.version.trim().to_owned(),
        }
    }
}
