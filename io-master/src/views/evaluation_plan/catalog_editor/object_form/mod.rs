//! The element "object" of the editor of a catalog: the group or control
//! chosen, its parts as the standard reads, and its own fields as JSON of
//! OSCAL, changed and saved here (page:
//! `ui/src/views/evaluation_plan/catalog_editor/ObjectForm.svelte`).
//!
//! ```text
//! action   command                     parameters             data back
//! read     object_form_get             uuid, id               ObjectView: the kind, the title, the parts, the own fields
//! save     object_form_save_group      uuid, id, group        (none)
//! save     object_form_save_control    uuid, id, control      (none)
//! delete   object_form_delete          uuid, id               (none); with what it holds and their quotes
//! ```
//!
//! A save changes only the own fields (id, title, class, props, parts,
//! params): the groups and controls the object holds stay. Every change is
//! checked against the schema of NIST when the catalog is stored.

pub mod commands;
pub mod view;
