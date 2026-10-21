//! The view Evaluation plan: the documents of the standards in NIST OSCAL
//! and the documents of standards (PDF) they quote (page:
//! `ui/src/views/plans/`). A catalog opens in its editor (`catalog_editor/`).
//!
//! ```text
//! element                  folder                commands
//! the documents of OSCAL   oscal_documents/      oscal_documents_get, _import, _export, _delete
//! a new catalog            new_catalog/          new_catalog_sources, new_catalog_create
//! the documents (PDF)      standard_documents/   standard_documents_get, _load, _stop, _update, _delete
//! the search               document_search/      document_search_run
//! the viewer               pdf_viewer/           pdf_viewer_documents, pdf_viewer_get
//! ```
//!
//! The flow of data. Which tab is open, which catalog is open and what the
//! viewer shows are presentation, kept by the page of the view. When an
//! element changed the documents it tells the view, which asks the lists
//! to read again:
//!
//! ```text
//! documents (PDF) ──load()──► standard_documents_load ──► pdf/: the text, part by part ──► db
//!      ▲                           └── LoadEvent (file, pages, loaded or failed, end) ──┘
//!      └── "changed" ──► the view ──► the lists read again
//! new catalog ──create(title, version, sources)──► new_catalog_create ──► db ──► "made" ──► the view opens it
//! search ──run(text)──► document_search_run ──► hits ──► "open" ──► the view ──► viewer ──get(id)──► the PDF
//! ```

pub mod document_search;
pub mod new_catalog;
pub mod oscal_documents;
pub mod pdf_viewer;
pub mod standard_documents;
