//! The element "instructions to the AI" of the editor of a catalog: the
//! instructions of the two tasks that build a catalog, as the evaluator
//! changed them, or the default ones (page:
//! `ui/src/views/evaluation_plan/catalog_editor/AiInstructions.svelte`).
//!
//! ```text
//! action   command                parameters      data back
//! read     ai_instructions_get    which           the instructions now in force
//! save     ai_instructions_save   which, text     the instructions now in force (empty text: the default)
//! ```
//!
//! `which` is `"catalog_outline"` (the groups) or `"catalog_section"` (the
//! controls). The instructions are evaluation support, kept in the
//! database of the evaluator (`ai/api/tasks.rs`).

pub mod commands;
