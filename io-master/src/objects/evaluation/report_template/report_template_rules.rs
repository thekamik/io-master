//! The rules of a template of a report: its markers and blocks, the report filled from it
//! (its values made harmless in the language of its file), its life (new, changed, imported,
//! written into a file).

pub mod escaping;
pub mod filling;
pub mod markers;
pub mod templates;
