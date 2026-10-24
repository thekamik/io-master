//! The events of a work of the AI, as the page gets them (steps 51.3, 51.4
//! and 52 of docs/todo.md): the proposals of the Evaluation plan, the
//! fragments, the assessment and the outputs of the Evaluation lab.

use serde::Serialize;

/// An event of a work of the AI.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum AiEvent {
    /// A part of the text starts (counted from 1).
    Part { index: usize, count: usize },
    /// The model is answering: the characters written so far.
    Writing { characters: usize },
    /// What an answer gave: the proposals added, the corrections, and what
    /// was dropped and why.
    Proposed { added: usize, corrections: usize, dropped: Vec<String> },
    /// A question gave nothing; why.
    Failed { message: String },
    /// The work is over.
    End { stopped: bool },
}
