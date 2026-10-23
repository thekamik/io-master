//! The element "documents of standards" of the view Evaluation plan: the
//! PDFs of the standards, loaded into the encrypted database with the
//! text of their pages (page: `ui/src/views/evaluation_plan/StandardDocuments.svelte`).
//!
//! ```text
//! action   command                      parameters           data back
//! read     standard_documents_get       (none)               the documents, by title and part
//! load     standard_documents_load      on_event (channel)   (none); LoadEvent while it works
//! stop     standard_documents_stop      (none)               (none); the loading ends after its part
//! change   standard_documents_update    id, document         DocumentView, as stored
//! delete   standard_documents_delete    id                   (none); refused while an OSCAL document quotes it
//! ```
//!
//! ```text
//! commands.rs   the five commands; the dialog of the system that chooses the files
//! load.rs       the loading, a long work, with its events
//! stored.rs     what is stored and its rules (a file once, the fields, a quoted document stays)
//! view.rs       DocumentView, DocumentForm, LoadEvent
//! ```
//!
//! The page never sees a path: the files are chosen in a dialog the
//! command opens, in Rust.

pub mod commands;
pub mod load;
pub mod stored;
pub mod view;
