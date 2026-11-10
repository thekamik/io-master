//! The progress of the long work of the AI ("Fill with AI", the assessment
//! of an evaluation): what the program shares with the page while it runs
//! and after it ended, so that a panel opened again shows it.
//!
//! **A helper model, never kept in a database** (user, 2026-11-01): it has
//! no table and no create, read or update in `db/`. The session holds the
//! one `Progress` of the program while it runs (`panels/session.rs`).

use crate::objects::evaluation::work_unit::work_unit_model::Verdict;

/// The state of the long work of the AI (user, 2026-11-01).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Progress {
    /// A long work of the AI runs.
    pub running: bool,

    /// The user asked it to stop: the answer it awaits ends at once, and no
    /// further step begins.
    pub stop: bool,

    /// The plan or the evaluation it works on (`EvaluationPlan`); `None`
    /// for a work of no plan (a question of a chat).
    pub evaluation_plan: Option<i64>,

    /// The work units to go through.
    pub total: usize,

    /// The work units done.
    pub assessed: usize,

    /// The work units not done: an error, a refused answer.
    pub failed: usize,

    /// The label of the last work unit gone through ("ADV_ARC.1-1").
    pub label: String,

    /// The verdict of the last work unit gone through; `None` when it has
    /// none (not assessed, or a work that gives no verdict).
    pub last_verdict: Option<Verdict>,

    /// The work units not taken or taken with notes, the newest first, at
    /// most `MAX_PROBLEMS` (`objects/session/progress/progress_ai/step.rs`).
    pub problems: Vec<Problem>,
}

/// A work unit not taken, or taken with notes (user, 2026-11-01).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    /// Its label.
    pub label: String,

    /// Why it was not taken, or what was not taken of its answer.
    pub notes: Vec<String>,
}
