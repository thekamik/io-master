//! The components of the window: small elements used more than once,
//! several times in one view or in several views (CLAUDE.md, "Terms";
//! docs/architecture.md, section 10.2). A component is built as an element
//! of a view (its `mod.rs`, `commands.rs`, `view.rs`), and its page is in
//! `ui/src/components/<component>/`.
//!
//! ```text
//! pdf_viewer/   the viewer of a standard: the evaluation plan, the editor of a catalog
//! ```
//!
//! The views use the components; a component never uses a view (checked by
//! `architecture_tests.rs`), and does its work with the folders below, as
//! the elements of a view do.

pub mod pdf_viewer;
