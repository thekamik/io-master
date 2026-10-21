//! The element "documents of OSCAL" of the view Evaluation plan: the
//! catalogs, mappings, profiles and plans of the evaluator, made in the
//! program or imported (page: `ui/src/views/plans/OscalDocuments.svelte`).
//!
//! ```text
//! action   command                  parameters         data back
//! read     oscal_documents_get      (none)             the documents, by model and title
//! import   oscal_documents_import   (none)             the document imported, or null (no file chosen)
//! export   oscal_documents_export   uuid, file_name    whether it was written (false: no place chosen)
//! delete   oscal_documents_delete   uuid               (none); its evaluation support with it
//! ```
//!
//! ```text
//! commands.rs   the four commands; the dialogs of the system
//! file.rs       a file of OSCAL read into the database, and written from it
//! view.rs       OscalDocumentView
//! ```
//!
//! Every document is checked against the schema of NIST when it is read
//! and again when it is stored or written (`model/oscal/schema.rs`).
//! Opening a catalog in its editor is presentation.

pub mod commands;
pub mod file;
pub mod view;
