//! The context of the standard for the agent of a requirement (step 57.4 of
//! docs/todo.md): the agent with knowledge of the task (its own, that of its
//! evaluation, or none), the passages of its bases that fit the requirement
//! and what the planner asks to check (`components/knowledge_agent/find.rs`),
//! and which of them the evaluator unticked. The ticked ones go into the
//! question of the assessment as context, never as evidence.

use crate::ai::tasks::candidates::requirement_text;
use crate::ai::tasks::knowledge_answer::passages_text;
use crate::app::error::AppError;
use crate::components::knowledge_agent::find;
use crate::db::evaluation as db_evaluation;
use crate::db::knowledge as db_knowledge;
use crate::db::user::database::UserDb;
use crate::model::evaluation::{LabTask, TaskAgent};
use crate::model::knowledge::{Agent, Hit, QUESTION_MAX};
use crate::model::task_scheme::BlockKind;

use super::super::lab_task::read as lab_task;

/// The context of a task: how its agent is chosen, the agent in force
/// (none when it has none or it was deleted), and its passages, each with
/// whether it is ticked.
pub struct Context {
    pub choice: TaskAgent,
    pub agent: Option<Agent>,
    pub passages: Vec<(Hit, bool)>,
}

/// The context of `task` of evaluation `evaluation`.
///
/// Called by: `lab_task/read.rs` (the screen), [`question_text`].
/// Calls: `db_evaluation::read::task_agent` and `agent`, `db_knowledge::read::agent`, [`query`],
/// `find::passages`, `db_evaluation::read::context_excluded`.
pub fn context(db: &UserDb, evaluation: &str, task: &LabTask) -> Result<Context, AppError> {
    let choice = db_evaluation::read::task_agent(db, evaluation, &task.uuid)?;
    let id = match choice {
        TaskAgent::AsEvaluation => db_evaluation::read::agent(db, evaluation)?,
        TaskAgent::NoAgent => None,
        TaskAgent::Agent(id) => Some(id),
    };
    let agent = match id {
        Some(id) => db_knowledge::read::agent(db, id)?,
        None => None,
    };
    let Some(agent) = agent else {
        return Ok(Context { choice, agent: None, passages: Vec::new() });
    };

    let (words, catalog_title) = query(db, evaluation, task)?;
    let hits = find::passages(db, &agent, &words)?;
    let excluded = db_evaluation::read::context_excluded(db, evaluation, &task.uuid)?;
    let mut passages = Vec::with_capacity(hits.len());

    for hit in hits {
        // The requirement itself, from its own catalog, is in the question already.
        if hit.passage.label == task.label && hit.source == catalog_title {
            continue;
        }

        let ticked = !excluded.contains(&hit.id);
        passages.push((hit, ticked));
    }

    Ok(Context { choice, agent: Some(agent), passages })
}

/// The words the passages are looked for with: the requirement (its label,
/// title and parts) and what the planner asks to check, at most
/// `QUESTION_MAX` characters (the search uses the first words only); and
/// the title of the catalog of the requirement.
///
/// Called by: [`context`].
/// Calls: `lab_task::requirement`, `requirement_text`, `db_evaluation::read::blocks`.
fn query(db: &UserDb, evaluation: &str, task: &LabTask) -> Result<(String, String), AppError> {
    let (control, _, catalog_title) = lab_task::requirement(db, task)?;
    let mut text = requirement_text(&task.label, &control).map_err(AppError::Invalid)?;

    for block in db_evaluation::read::blocks(db, evaluation, &task.uuid)? {
        if block.kind == BlockKind::Requirement {
            text.push_str(&block.fields.instructions);
        }
    }

    Ok((text.chars().take(QUESTION_MAX).collect(), catalog_title))
}

/// The context for the question of the assessment: the instructions of the
/// agent and its ticked passages, numbered (`passages_text`); empty when
/// the task has no agent or no passage is ticked.
///
/// Called by: `assessment/propose.rs`.
/// Calls: [`context`], `passages_text`.
pub fn question_text(db: &UserDb, evaluation: &str, task: &LabTask) -> Result<String, AppError> {
    let found = context(db, evaluation, task)?;
    let Some(agent) = found.agent else {
        return Ok(String::new());
    };
    let mut ticked = Vec::with_capacity(found.passages.len());

    for (hit, chosen) in found.passages {
        if chosen {
            ticked.push(hit);
        }
    }

    if ticked.is_empty() {
        return Ok(String::new());
    }

    Ok(format!(
        "The agent \"{}\": {}\n\n{}",
        agent.fields.name,
        agent.fields.instructions.trim(),
        passages_text(&ticked)
    ))
}
