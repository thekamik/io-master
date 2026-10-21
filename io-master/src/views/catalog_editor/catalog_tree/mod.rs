//! The element "tree" of the editor of a catalog: its groups and controls,
//! as rows with their level, so that the page draws the tree without
//! recursion (page: `ui/src/views/catalog_editor/CatalogTree.svelte`).
//!
//! ```text
//! action   command            parameters   data back
//! read     catalog_tree_get   uuid         CatalogTreeView: the title, the version, the last change, the rows
//! ```
//!
//! Choosing an object is presentation: the page tells the view.

pub mod commands;
pub mod view;
