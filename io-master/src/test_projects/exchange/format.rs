//! The format of the file of an exported test project, so that a head of the
//! laboratory can give a project to the evaluators (steps 22 and 25 of
//! `docs/todo.md`).
//!
//! The file is JSON:
//!
//! ```text
//! {
//!   "format": "io-master-project", "version": 1,
//!   "project": {"name": ..., "version": ..., "description": ..., "author": ...},
//!   "documents": [{"key": "d1", "title": ..., "part": ..., "version": ...,
//!                  "file_name": ..., "sha256": ..., "page_count": 236,
//!                  "pdf": "<base64>"}],          "pdf" only if chosen
//!   "schemes": [{"key": "s1", "number": "9.1", "name": ..., "state": "accepted",
//!                "pages": null | {"document": "d1", "first_page": 40, "last_page": 52}}],
//!   "tiles": [{"key": "t1", "scheme": "s1", "kind": "requirement", "identifier": ..., "title": ...,
//!              "description": ..., "place": ..., "state": "accepted",
//!              "external": null | {"document": ..., "place": ...},
//!              "sections": [...], "instructions": ..., "x": 0, "y": 0,
//!              "sources": [{"document": "d1", "page": 12, "quote": ...}]}],
//!   "links": [{"from": "t1", "to": "t2"}]
//! }
//! ```
//!
//! Schemes, tiles and documents are named by keys of the file, not by the
//! numbers of a database: the numbers differ on every installation.
//! A document is found again by its SHA-256; the PDF travels in the
//! file only when the user chooses so (the laboratory decides, user,
//! 2026-10-16). Whether a quote is found is not taken from the file:
//! it is checked again against the copy of the importer.
//!
//! The JSON is built and read by hand here, as a plain value, so that
//! the types of the core carry no serialization code (CLAUDE.md).
//!
//! ```text
//! format.rs   the format: its name, version, the types of a file read
//! export.rs   a project as a file
//! import.rs   a file read and checked; a new project stored from it
//! ```

use std::fmt;

use crate::test_projects::model::{Link, ProjectFields, Scheme, Tile};

/// The name of the format, in every file.
pub const FORMAT: &str = "io-master-project";

/// The version of the format this program writes and reads.
pub const VERSION: u64 = 1;

/// The largest file of a project that is read, in bytes.
pub const MAX_FILE: usize = 512 * 1024 * 1024;

/// A document named in a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExchangeDocument {
    pub key: String,
    pub title: String,
    pub part: String,
    pub version: String,
    pub file_name: String,
    pub sha256: String,
    pub page_count: u32,
    /// The PDF, when the file carries it.
    pub pdf: Option<Vec<u8>>,
}

/// A project read from a file, before it is stored.
#[derive(Debug, Clone, PartialEq)]
pub struct Exchange {
    /// The fields of the project, without documents.
    pub project: ProjectFields,
    pub documents: Vec<ExchangeDocument>,
    /// The schemes, numbered 1, 2, ... in the order of the file; their
    /// pages name documents by their place in `documents` + 1.
    pub schemes: Vec<Scheme>,
    /// The tiles, numbered 1, 2, ... in the order of the file; their
    /// scheme is the number of a scheme above, the sources name
    /// documents by their place in `documents` + 1.
    pub tiles: Vec<Tile>,
    pub links: Vec<Link>,
}

/// Why a file could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExchangeError {
    /// Not a file of a project, or a broken one; holds what is wrong.
    Malformed(String),
    /// A newer version of the format.
    NewerVersion(u64),
    /// The PDF of a document carried in the file is not the one it
    /// names.
    WrongPdf(String),
}

impl fmt::Display for ExchangeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExchangeError::Malformed(reason) => write!(formatter, "the file is not a valid test project: {reason}"),
            ExchangeError::NewerVersion(version) => write!(
                formatter,
                "the file is of version {version} of the format; this program reads version {VERSION}; update it"
            ),
            ExchangeError::WrongPdf(key) => write!(formatter, "the PDF carried for {key} is not the document it names"),
        }
    }
}

impl std::error::Error for ExchangeError {}
