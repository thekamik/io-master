//! The instructions of the program for a task of the AI: the fixed part
//! of every question of the task ("assess this work unit, answer in this
//! form ..."), which the user may change. They are given with the
//! instructions of the agent, which say who the agent is. Kept in the
//! table `ai_instructions`, one row per task.

/// The instructions of a task of the AI (user, 2026-10-28).
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code, reason = "a new model (2026-10-28), not used by the program yet, only by db/ and its tests")]
pub struct Instructions {
    /// The task they are for.
    pub task: AiTask,

    /// The text of the instructions.
    pub text: String,
}

/// A task of the AI (user, 2026-10-28).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code, reason = "a new model (2026-10-28), not used by the program yet, only by db/ and its tests")]
pub enum AiTask {
    /// The assessment of a work unit: its results, its quotes and its
    /// verdict.
    Assessment,
    /// A consultation: a message of a chat.
    Consultation,
}
