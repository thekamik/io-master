//! The instructions of the tasks of the AI, as the page calls them: both
//! read, one saved.
//!
//! ```text
//! Instructions.svelte ──instructions_get()──────────► task_instructions, every task ──► [InstructionsView]
//!                     ──instructions_save(task, text)► save_instructions
//! ```

use serde::Serialize;
use tauri::State;

use crate::db::ai::instructions::MAX_INSTRUCTIONS;
use crate::model::ai::instructions::{AiTask, Instructions};
use crate::panels::logic::settings::instructions::{TASKS, save_instructions, task_instructions};
use crate::panels::views::error::ViewError;
use crate::panels::views::opened::database;
use crate::session::state::Session;

/// The instructions of a task, as the page shows them.
#[derive(Serialize)]
pub struct InstructionsView {
    /// "assessment", "consultation".
    task: &'static str,
    text: String,
}

/// The name of a task on the page.
///
/// Called by: [`instructions_get`].
fn task_name(task: AiTask) -> &'static str {
    match task {
        AiTask::Assessment => "assessment",
        AiTask::Consultation => "consultation",
    }
}

/// The task of its name on the page; a name the page never sends is refused.
///
/// Called by: [`instructions_save`].
fn task_of(name: &str) -> Result<AiTask, ViewError> {
    match name {
        "assessment" => Ok(AiTask::Assessment),
        "consultation" => Ok(AiTask::Consultation),
        _ => Err(ViewError::Invalid(format!("no task \"{name}\""))),
    }
}

/// The instructions of every task: the stored ones, else the default ones.
///
/// Called by: `Instructions.svelte`, when the tab opens.
/// Calls: `task_instructions`.
#[tauri::command]
pub fn instructions_get(session: State<'_, Session>) -> Result<Vec<InstructionsView>, ViewError> {
    let opened = session.lock()?;
    let db = database(&opened)?;
    let mut views = Vec::with_capacity(TASKS.len());

    for task in TASKS {
        views.push(InstructionsView { task: task_name(task), text: task_instructions(db, task)?.text });
    }

    Ok(views)
}

/// Saves the instructions of the task `task`.
///
/// Called by: `Instructions.svelte`, "Save".
/// Calls: `save_instructions`.
#[tauri::command]
pub fn instructions_save(session: State<'_, Session>, task: String, text: String) -> Result<(), ViewError> {
    // Four bytes at most per character: a longer text is over the limit.
    ensure!(
        text.len() <= MAX_INSTRUCTIONS * 4,
        ViewError::Invalid(format!("instructions over {MAX_INSTRUCTIONS} characters"))
    );

    let instructions = Instructions { task: task_of(&task)?, text };
    let opened = session.lock()?;

    Ok(save_instructions(database(&opened)?, &instructions)?)
}
