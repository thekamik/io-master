//! The data of the component "agent with knowledge", as the page gets it:
//! the agents that may be chosen for a work of the AI.

use serde::Serialize;

use crate::app::error::AppError;
use crate::db_old::knowledge as db_knowledge;
use crate::db_old::user::database::UserDb;

/// An agent with knowledge that may be chosen.
#[derive(Debug, Serialize)]
pub struct AgentOptionView {
    pub id: i64,
    pub name: String,
}

/// The agents of the evaluator, by name.
///
/// Called by: `knowledge_agent_list` (`knowledge_agent.rs`), the screens of the Evaluation lab.
/// Calls: `db_knowledge::read::agents`.
pub fn agents(db: &UserDb) -> Result<Vec<AgentOptionView>, AppError> {
    let summaries = db_knowledge::read::agents(db)?;
    let mut options = Vec::with_capacity(summaries.len());

    for summary in summaries {
        options.push(AgentOptionView { id: summary.id, name: summary.name });
    }

    Ok(options)
}
