//! The agents of the panel of the agents, as the page calls them: the list,
//! a new agent, one read and saved, and the test of its knowledge.
//!
//! ```text
//! AgentList.svelte   ──agent_list()──────────────► read_agents ──► [AgentSummaryView]
//!                    ──agent_create(name, integration)► new_agent ──► id
//! AgentEditor.svelte ──agent_get(id)─────────────► the agent, the integrations and bases to choose ──► AgentView
//!                    ──agent_save(fields)────────► change_agent
//!                    ──agent_knowledge(id, question)► agent_knowledge ──► the passages it would give
//! ```

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::db::models::ai::agent::{read_agent, read_agents};
use crate::db::models::ai::integration::{read_integration, read_integrations};
use crate::db::models::ai::rag::read_knowledge_bases;
use crate::model::ai::agent::{Agent, GlossaryEntry};
use crate::panels::logic::agents::agent::{MAX_INSTRUCTIONS, agent_knowledge, change_agent, new_agent};
use crate::panels::views::error::ViewError;
use crate::panels::views::opened::database;
use crate::session::state::Session;

/// The longest name or question from the page, in bytes.
const MAX_FIELD: usize = 4096;

/// The most terms of a glossary from the page, and the longest term or
/// meaning, in bytes (the logic checks the characters).
const MAX_TERMS: usize = 500;
const MAX_TERM_BYTES: usize = 2048;

/// An agent, as the list shows it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSummaryView {
    id: i64,
    name: String,
    /// The name of its integration and its model.
    integration: String,
    knowledge_bases: usize,
}

/// A choice of the editor: an integration or a knowledge base.
#[derive(Serialize)]
pub struct ChoiceView {
    id: i64,
    name: String,
}

/// A term of a glossary, as the page shows and sends it.
#[derive(Serialize, Deserialize)]
pub struct TermView {
    term: String,
    meaning: String,
}

/// An agent whole, with what its editor offers to choose.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentView {
    id: i64,
    name: String,
    integration: i64,
    instructions: String,
    knowledge_bases: Vec<i64>,
    glossary: Vec<TermView>,
    max_passages: u32,
    max_characters: u32,
    /// Every integration of the user ("name · model").
    integrations: Vec<ChoiceView>,
    /// Every knowledge base of the user.
    bases: Vec<ChoiceView>,
}

/// The fields of an agent as the page sends them.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentFields {
    id: i64,
    name: String,
    integration: i64,
    instructions: String,
    knowledge_bases: Vec<i64>,
    glossary: Vec<TermView>,
    max_passages: u32,
    max_characters: u32,
}

/// Every agent, by its name.
///
/// Called by: `AgentList.svelte`, when the panel opens and after a change.
/// Calls: `read_agents`, `read_integration`.
#[tauri::command]
pub fn agent_list(session: State<'_, Session>) -> Result<Vec<AgentSummaryView>, ViewError> {
    let opened = session.lock()?;
    let db = database(&opened)?;
    let mut agents = Vec::new();

    for agent in read_agents(db)? {
        let integration = read_integration(db, agent.integration)?;

        agents.push(AgentSummaryView {
            id: agent.id,
            name: agent.name,
            integration: format!("{} · {}", integration.name, integration.model),
            knowledge_bases: agent.knowledge_bases.len(),
        });
    }

    Ok(agents)
}

/// Makes a new agent of the integration `integration`; gives its id.
///
/// Called by: `AgentList.svelte`, "Make the agent".
/// Calls: `new_agent`.
#[tauri::command]
pub fn agent_create(session: State<'_, Session>, name: String, integration: i64) -> Result<i64, ViewError> {
    ensure!(name.len() <= MAX_FIELD, ViewError::Invalid("the name is too long".to_owned()));

    let opened = session.lock()?;

    Ok(new_agent(database(&opened)?, &name, integration)?)
}

/// The agent `id` whole, with every integration and knowledge base to
/// choose from.
///
/// Called by: `AgentEditor.svelte`, when an agent is opened and after a save.
/// Calls: `read_agent`, `read_integrations`, `read_knowledge_bases`.
#[tauri::command]
pub fn agent_get(session: State<'_, Session>, id: i64) -> Result<AgentView, ViewError> {
    let opened = session.lock()?;
    let db = database(&opened)?;
    let agent = read_agent(db, id)?;
    let mut integrations = Vec::new();
    let mut bases = Vec::new();
    let mut glossary = Vec::with_capacity(agent.glossary.len());

    for integration in read_integrations(db)? {
        integrations
            .push(ChoiceView { id: integration.id, name: format!("{} · {}", integration.name, integration.model) });
    }

    for base in read_knowledge_bases(db)? {
        bases.push(ChoiceView { id: base.id, name: base.name });
    }

    for entry in agent.glossary {
        glossary.push(TermView { term: entry.term, meaning: entry.meaning });
    }

    Ok(AgentView {
        id: agent.id,
        name: agent.name,
        integration: agent.integration,
        instructions: agent.instructions,
        knowledge_bases: agent.knowledge_bases,
        glossary,
        max_passages: agent.max_passages,
        max_characters: agent.max_characters,
        integrations,
        bases,
    })
}

/// The fields of an agent from the page, checked for their size, as the
/// model.
///
/// Called by: [`agent_save`].
fn agent_of(fields: AgentFields) -> Result<Agent, ViewError> {
    ensure!(fields.name.len() <= MAX_FIELD, ViewError::Invalid("the name is too long".to_owned()));
    ensure!(
        fields.instructions.len() <= MAX_INSTRUCTIONS * 4,
        ViewError::Invalid(format!("instructions over {MAX_INSTRUCTIONS} characters"))
    );
    ensure!(fields.glossary.len() <= MAX_TERMS, ViewError::Invalid(format!("more than {MAX_TERMS} terms")));

    let mut glossary = Vec::with_capacity(fields.glossary.len());

    for entry in fields.glossary {
        ensure!(
            entry.term.len() <= MAX_TERM_BYTES && entry.meaning.len() <= MAX_TERM_BYTES,
            ViewError::Invalid("a term or a meaning is too long".to_owned())
        );
        glossary.push(GlossaryEntry { term: entry.term, meaning: entry.meaning });
    }

    Ok(Agent {
        id: fields.id,
        name: fields.name,
        integration: fields.integration,
        instructions: fields.instructions,
        knowledge_bases: fields.knowledge_bases,
        glossary,
        max_passages: fields.max_passages,
        max_characters: fields.max_characters,
    })
}

/// Saves the agent `fields.id`.
///
/// Called by: `AgentEditor.svelte`, "Save".
/// Calls: [`agent_of`], `change_agent`.
#[tauri::command]
pub fn agent_save(session: State<'_, Session>, fields: AgentFields) -> Result<(), ViewError> {
    let agent = agent_of(fields)?;
    let opened = session.lock()?;

    Ok(change_agent(database(&opened)?, &agent)?)
}

/// The knowledge the agent `id` would give with the question `question`:
/// the test of its bases, its glossary and its limits, before it works on
/// a work unit. Nothing is sent to an AI service.
///
/// Called by: `AgentEditor.svelte`, "Show the knowledge".
/// Calls: `agent_knowledge`.
#[tauri::command]
pub fn agent_knowledge_test(session: State<'_, Session>, id: i64, question: String) -> Result<Vec<String>, ViewError> {
    ensure!(question.len() <= MAX_FIELD, ViewError::Invalid("the question is too long".to_owned()));

    let opened = session.lock()?;

    Ok(agent_knowledge(database(&opened)?, id, &question)?)
}
