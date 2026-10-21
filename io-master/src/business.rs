//! The business logic of the program (step 40 of docs/todo.md): what
//! the evaluator does, as resources and operations, reached by the
//! window (`tauri_ui/`) and by nothing else. It uses `ai/` to ask the
//! AI, `hardware_tests/` to reach devices and `db/` to store; it never
//! presents anything itself.
//!
//! ```text
//! standards/       the documents of standards (PDF): the sources
//! oscal/           the documents of NIST OSCAL: the standards, the plans, the evaluations
//! chats/           the chats with the AI
//! work.rs          what the long works share: stop, error, steps
//! ```

pub mod chats;
pub mod oscal;
pub mod standards;
pub mod work;
