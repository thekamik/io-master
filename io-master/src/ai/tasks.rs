//! The tasks of the AI: for each, the question sent to the model and
//! how its answer is read (step 40 of docs/todo.md). Everything that is
//! said to a model, and every reading of what it says, is here, in one
//! place for the audit. A task works on plain data (`model/`) and gives
//! back plain data: applying a proposal to a project is the work of
//! the business (step 41: the catalogs of OSCAL).
//!
//! ```text
//! chapter.rs    the analysis of a chapter: tiles and links
//! contents.rs   the table of contents: the chapters
//! chat.rs       the chat: what of its history goes to the model
//! json.rs       the JSON object in an answer
//! ```

pub mod chat;
