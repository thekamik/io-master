//! The element "proposals" of the editor of a catalog: what the AI
//! proposed, waiting for the evaluator, who accepts it (as it is or
//! corrected) or rejects it (page: `ui/src/views/evaluation_plan/catalog_editor/Proposals.svelte`).
//!
//! ```text
//! action   command            parameters          data back
//! read     proposals_get      uuid                the proposals, the oldest first, with their parts and quotes
//! accept   proposals_accept   id, corrected       (none); the object enters the catalog
//! reject   proposals_reject   id                  (none); the catalog is not touched
//! ```
//!
//! A proposal enters the catalog only when the evaluator accepts it (P1 of
//! docs/architecture.md). In one transaction: the object added (checked
//! against the schema of NIST), its quotes kept as evaluation support, the
//! proposal removed (`accept.rs`).

pub mod accept;
pub mod commands;
pub mod view;
