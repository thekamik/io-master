//! The views of the window, one folder per view and one folder per
//! interactive element in it (`docs/code_structure.md`, section "Views").
//!
//! An element of the page that needs the program (to process or store
//! data, to scan, to ask the AI) has here its own folder with the
//! commands it calls and the data it gets back:
//!
//! ```text
//! views/<view>/mod.rs              the view: its elements and the flow of data between them
//! views/<view>/<element>/mod.rs    the element: its actions, their parameters, their data
//!                     commands.rs  the commands of the element: parameters -> work -> data
//!                     view.rs      the data the element gets (serde: only here and in tauri_ui/)
//! ```
//!
//! The page has the element of the same name (`ui/src/views/<view>/`).
//! A command does its work by calling the folders below it (`model/`,
//! `db/`, `ai/`, `hardware_tests/`) and nothing in between; what two
//! elements need goes down to those folders, or, when it only joins
//! them, to a file of the view.
//!
//! The views replace `business/` and the commands of `tauri_ui/`
//! (step 42 of docs/todo.md); until every view is moved, both live
//! beside this folder.

pub mod chat;
pub mod scan;
pub mod settings;
pub mod shell;
pub mod start;
#[cfg(test)]
pub mod test_support;
