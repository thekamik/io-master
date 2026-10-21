//! The element "new catalog" of the view Evaluation plan: a catalog of
//! OSCAL made from documents of a standard (page:
//! `ui/src/views/plans/NewCatalog.svelte`).
//!
//! ```text
//! action    command               parameters                 data back
//! sources   new_catalog_sources   (none)                     the documents it can come from
//! make      new_catalog_create    title, version, sources    OscalDocumentView of the new catalog
//! ```
//!
//! The catalog is empty; its groups and controls are made in its editor,
//! by hand or with the AI. The documents it comes from are resources of
//! its back matter (`business/oscal/catalogs.rs`, moved with the editor).

pub mod commands;
pub mod view;
