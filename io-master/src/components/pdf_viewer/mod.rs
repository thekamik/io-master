//! The component "viewer of a standard": the original page of a PDF, with a
//! quote highlighted in it, drawn by pdf.js in the page (page:
//! `ui/src/components/pdf_viewer/`). Used by the view Evaluation plan and
//! by its editor of a catalog.
//!
//! ```text
//! action      command                parameters   data back
//! documents   pdf_viewer_documents   (none)       the documents it can show: title, part, pages
//! read        pdf_viewer_get         id           the PDF, as bytes (an ArrayBuffer in the page)
//! ```
//!
//! The PDF comes from the encrypted database; the page, the zoom and the
//! selection of a quote are presentation.

pub mod commands;
pub mod view;
