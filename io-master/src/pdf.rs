//! Reading PDFs with the tools of Poppler (`pdfinfo`, `pdftotext`): the
//! only place where the program starts another program on a document of
//! the evaluator, kept in one folder for the audit.
//!
//! ```text
//! poppler.rs   the number of pages and the title, the text of a range of pages
//! ```
//!
//! It uses nothing of the program; the views call it (`views/plans/`).

pub mod poppler;
