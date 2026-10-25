//! The data of the form of an agent, as the page gets it and sends it: its
//! fields, and every knowledge base, ticked when the agent draws on it, so
//! that the evaluator joins an agent to its bases in this one form.

use serde::{Deserialize, Serialize};

use crate::app::error::AppError;
use crate::db::knowledge as db_knowledge;
use crate::db::user::database::UserDb;
use crate::model::knowledge::{
    Agent, AgentFields, DEFAULT_AGENT_PASSAGES, DEFAULT_CONTEXT_CHARS, MAX_AGENT_PASSAGES, MAX_CONTEXT_CHARS,
    MIN_CONTEXT_CHARS,
};

/// The instructions of a new agent.
pub const NEW_INSTRUCTIONS: &str = "You help an evaluator of a security evaluation. You know the standard from the passages given with every question.";

/// A knowledge base in the form of an agent: ticked when the agent draws on it.
#[derive(Debug, Serialize)]
pub struct BaseChoiceView {
    pub id: i64,
    pub name: String,
    pub passages: usize,
    pub chosen: bool,
}

/// The form of an agent, with the bounds of its numbers.
#[derive(Debug, Serialize)]
pub struct AgentFormView {
    pub id: i64,
    pub name: String,
    pub instructions: String,
    pub glossary: String,
    pub passages: usize,
    pub characters: usize,
    pub bases: Vec<BaseChoiceView>,
    pub max_passages: usize,
    pub min_characters: usize,
    pub max_characters: usize,
}

/// The form of an agent as the evaluator sends it.
#[derive(Debug, Deserialize)]
pub struct AgentForm {
    pub name: String,
    pub instructions: String,
    pub glossary: String,
    pub passages: usize,
    pub characters: usize,
    pub bases: Vec<i64>,
}

impl AgentForm {
    /// The fields of the agent, the name trimmed.
    ///
    /// Called by: `agent_form_save` (`knowledge.rs`).
    pub fn fields(self) -> AgentFields {
        AgentFields {
            name: self.name.trim().to_owned(),
            instructions: self.instructions,
            glossary: self.glossary,
            passages: self.passages,
            characters: self.characters,
            bases: self.bases,
        }
    }
}

/// The fields of a new agent named `name`: the default instructions and
/// limits, no base yet.
///
/// Called by: `agent_list_create` (`knowledge.rs`).
pub fn new_agent(name: &str) -> AgentFields {
    AgentFields {
        name: name.trim().to_owned(),
        instructions: NEW_INSTRUCTIONS.to_owned(),
        glossary: String::new(),
        passages: DEFAULT_AGENT_PASSAGES,
        characters: DEFAULT_CONTEXT_CHARS,
        bases: Vec::new(),
    }
}

/// The form of `agent`, with every base of the evaluator.
///
/// Called by: `agent_form_get`, `agent_form_save`, `agent_list_create` (`knowledge.rs`).
/// Calls: `db_knowledge::read::bases`.
pub fn form(db: &UserDb, agent: Agent) -> Result<AgentFormView, AppError> {
    let summaries = db_knowledge::read::bases(db)?;
    let mut bases = Vec::with_capacity(summaries.len());

    for summary in summaries {
        let chosen = agent.fields.bases.contains(&summary.base.id);
        bases.push(BaseChoiceView { id: summary.base.id, name: summary.base.name, passages: summary.passages, chosen });
    }

    Ok(AgentFormView {
        id: agent.id,
        name: agent.fields.name,
        instructions: agent.fields.instructions,
        glossary: agent.fields.glossary,
        passages: agent.fields.passages,
        characters: agent.fields.characters,
        bases,
        max_passages: MAX_AGENT_PASSAGES,
        min_characters: MIN_CONTEXT_CHARS,
        max_characters: MAX_CONTEXT_CHARS,
    })
}
