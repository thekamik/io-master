//! The element "quotes" of the editor of a catalog: the passages of the
//! documents of the standard the object chosen comes from, each looked
//! for on its page (page: `ui/src/views/evaluation_plan/catalog_editor/ObjectQuotes.svelte`).
//!
//! ```text
//! action   command                 parameters                              data back
//! read     object_quotes_get       uuid, object_id                         ObjectQuotesView: the quotes, the first source
//! add      object_quotes_add       uuid, object_id, source, page, text     (none)
//! delete   object_quotes_delete    id                                      (none)
//! ```
//!
//! A quote is taken in the viewer of a standard (text selected in its
//! page); the viewer is presentation of the view. The quotes are
//! evaluation support (`db/evaluation_support/`), linked to the catalog by
//! its UUID and to the object by its id.

pub mod commands;
pub mod view;
