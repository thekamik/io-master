//! The component "parts of an object": the parts of a group or a control
//! of OSCAL (statement, items, guidance, ...) as rows with their level, so
//! that the page shows them indented, without recursion (page:
//! `ui/src/components/part_rows/`). Used twice by the editor of a catalog:
//! for the object chosen (`object_form/`) and for the proposals of the AI
//! (`proposals/`).
//!
//! ```text
//! view.rs   the rows, made from the parts (no command: the elements that show
//!           an object give its rows with their own data)
//! ```

pub mod view;
