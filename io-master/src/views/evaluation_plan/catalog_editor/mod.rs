//! The editor of a catalog, a part of the view Evaluation plan: a catalog of
//! OSCAL made in the program from the documents (PDF) of its standard, group
//! by group and control by control, with the AI (page:
//! `ui/src/views/evaluation_plan/catalog_editor/`; opened from the list of
//! the documents of OSCAL).
//!
//! ```text
//! element             folder             commands
//! the tree            catalog_tree/      catalog_tree_get
//! the object chosen   object_form/       object_form_get, _save_group, _save_control, _delete
//! a new object        new_object/        new_object_add_group, new_object_add_control
//! its quotes          object_quotes/     object_quotes_get, _add, _delete
//! the proposals       proposals/         proposals_get, _accept, _reject
//! the AI              ai_builder/        ai_builder_sources, _groups, _controls, _stop
//! its instructions    ai_instructions/   ai_instructions_get, ai_instructions_save
//! ```
//!
//! The flow of data. Which object is chosen, and what the viewer shows,
//! are presentation, kept by the page of the view. Every element that
//! changes the catalog or its evaluation support tells the view, which
//! asks every element to read its data again:
//!
//! ```text
//! tree ──get(uuid)──► catalog_tree_get ──► the rows of the tree (no recursion in the page)
//!   └── "chosen" ──► the view ──► form ──get(uuid, id)──► object_form_get; quotes ──get──► object_quotes_get
//! form, new object, quotes, proposals ──a change──► the catalog in db/oscal/ (checked against the schema of NIST)
//!   └── "changed" ──► the view ──► every element reads again
//! AI ──groups / controls(pages)──► ai_builder_* ──► ai/api/: the model, part by part ──► proposals in db/
//!   └── BuildEvent (part, writing, proposed, failed, end) while it works; "changed" at the end
//! ```
//!
//! Nothing the AI proposes enters the catalog until the evaluator accepts
//! it, as it is or corrected (P1 of docs/architecture.md).
//!
//! ```text
//! checked_quote.rs   a quote looked for on its page (the quotes, the proposals of the AI)
//! oscal_object.rs    a group or a control sent by the page, read into the types of OSCAL
//! ```

pub mod ai_builder;
pub mod ai_instructions;
pub mod catalog_tree;
pub mod checked_quote;
pub mod new_object;
pub mod object_form;
pub mod object_quotes;
pub mod oscal_object;
pub mod proposals;
#[cfg(test)]
pub mod test_support;
