//! The element "the AI" of the editor of a catalog: the AI reads a table
//! of contents and proposes the groups, or the pages of a group and
//! proposes its controls (page: `ui/src/views/evaluation_plan/catalog_editor/AiBuilder.svelte`).
//!
//! ```text
//! action     command               parameters                                   data back
//! sources    ai_builder_sources    (none)                                       the documents it can read
//! groups     ai_builder_groups     uuid, range, on_event                        (none); BuildEvent while it works
//! controls   ai_builder_controls   uuid, group, ranges, part_size, on_event     (none); BuildEvent while it works
//! stop       ai_builder_stop       (none)                                       (none); the work ends after its step
//! ```
//!
//! Every question follows one pipeline:
//!
//! ```text
//! the catalog and the pages (db/)  ──prompt (ai/tasks/catalog_*.rs)──►  the question
//!   ──ai/api/completions::ask──►  the model, and the journal of the AI
//!   ──read (ai/tasks/catalog_*.rs)──►  groups or controls of OSCAL
//!   ──quotes checked on their pages, proposals stored (db/evaluation_support/)
//! ```
//!
//! What the AI proposes is never written into the catalog: it waits as a
//! proposal until the evaluator accepts it (`proposals/`).
//!
//! ```text
//! commands.rs   the four commands
//! groups.rs     the groups from a table of contents
//! controls.rs   the controls of a group from its pages
//! steps.rs      what both share: the model, the parts, one question, the quotes
//! parts.rs      the pages cut into parts that fit the context of a model
//! view.rs       PageRange, SourceView, BuildEvent
//! ```
//!
//! The database is held for each step only, never while the model writes.

pub mod commands;
pub mod controls;
pub mod groups;
pub mod parts;
pub mod steps;
pub mod view;
