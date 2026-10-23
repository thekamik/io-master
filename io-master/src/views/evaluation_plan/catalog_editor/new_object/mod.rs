//! The element "new object" of the editor of a catalog: a dialog that adds
//! a group or a control, written as JSON of OSCAL, at a place of the
//! catalog (page: `ui/src/views/evaluation_plan/catalog_editor/NewObject.svelte`).
//!
//! ```text
//! action   command                  parameters             data back
//! add      new_object_add_group     uuid, place, group     (none)
//! add      new_object_add_control   uuid, place, control   (none)
//! ```
//!
//! A place is the top of the catalog, a group, or a control (for an
//! enhancement); a group goes only into the catalog or a group. The new
//! object comes without what it would hold: that is added later.

pub mod commands;
pub mod view;
