//! The element "viewer of a standard" of the view Evaluation plan: the
//! original page of a PDF, with a quote highlighted in it, drawn by pdf.js
//! in the page (page: `ui/src/views/evaluation_plan/PdfViewer.svelte`; the editor of
//! a catalog uses it too).
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
