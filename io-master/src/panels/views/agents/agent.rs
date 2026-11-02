//! The agents of the panel of the agents, as the page calls them: the list,
//! a new agent, one read and saved, and the test of its knowledge.
//!
//! ```text
//! AgentList.svelte   ──agent_list()──────────────► read_agents ──► [AgentSummaryView]
//!                    ──agent_create(name, integration, role)► new_agent ──► id
//! AgentEditor.svelte ──agent_get(id)─────────────► the agent, the integrations and bases to choose ──► AgentView
//!                    ──agent_save(fields)────────► change_agent
//!                    ──agent_knowledge(id, question)► agent_knowledge ──► the passages it would give
//! ```

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::objects::ai::agent::agent_ai::agent_knowledge;
use crate::objects::ai::agent::agent_model::{Agent, GlossaryEntry};
use crate::objects::ai::agent::agent_rules::{MAX_INSTRUCTIONS, change_agent, new_agent};
use crate::objects::ai::agent::agent_store::{read_agent, read_agents};
use crate::objects::ai::agent_roles::agent_roles_model::{AgentRole, EvaluatorRole, PlannerRole};
use crate::objects::ai::agent_roles::agent_roles_store::{MAX_TASK, role_name};
use crate::objects::ai::integration::integration_store::{read_integration, read_integrations};
use crate::objects::ai::rag::rag_store::read_knowledge_bases;
use crate::panels::session::Session;
use crate::panels::views::error::ViewError;
use crate::panels::views::opened::database;

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
    /// `"evaluator"`, `"planner"`, `"chat"`.
    role: &'static str,
}

/// The role of an agent as the page shows and sends it: its kind and the
/// parameters of every role, those of another role left empty (user,
/// 2026-11-02).
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleView {
    /// `"evaluator"`, `"planner"`, `"chat"`.
    kind: String,
    judgement: String,
    documentation_task: String,
    functional_task: String,
    document_passages: u32,
    dependency_findings: u32,
    parameters_task: String,
    dependencies_task: String,
    summary_characters: u32,
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
    max_answer_tokens: u32,
    role: RoleView,
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
    max_answer_tokens: u32,
    role: RoleView,
}

/// The role of an agent as the page shows it.
///
/// Input: `role`, the role of a stored agent.
/// Output: its view (`RoleView { kind: "evaluator", document_passages: 20, .. }`),
/// the fields of another role empty.
///
/// Called by: [`agent_get`].
/// Calls: `objects/ai/agent_roles/agent_roles_store.rs`: `role_name`.
fn role_view(role: AgentRole) -> RoleView {
    let mut view = RoleView {
        kind: role_name(&role).to_owned(),
        judgement: String::new(),
        documentation_task: String::new(),
        functional_task: String::new(),
        document_passages: 0,
        dependency_findings: 0,
        parameters_task: String::new(),
        dependencies_task: String::new(),
        summary_characters: 0,
    };

    match role {
        AgentRole::Evaluator(evaluator) => {
            view.judgement = evaluator.judgement;
            view.documentation_task = evaluator.documentation_task;
            view.functional_task = evaluator.functional_task;
            view.document_passages = evaluator.document_passages;
            view.dependency_findings = evaluator.dependency_findings;
        }
        AgentRole::Planner(planner) => {
            view.parameters_task = planner.parameters_task;
            view.dependencies_task = planner.dependencies_task;
            view.summary_characters = planner.summary_characters;
        }
        AgentRole::Chat => {}
    }

    view
}

/// The role of an agent from the page, checked for the size of its texts, as
/// the model: the fields of its kind taken, those of another kind passed over.
///
/// Input: `view`, the role as the editor sends it.
/// Output: the role (`AgentRole::Planner(PlannerRole { .. })`);
/// `ViewError::Invalid` for a text over its size or a kind unknown.
///
/// Called by: [`agent_of`].
fn role_of(view: RoleView) -> Result<AgentRole, ViewError> {
    for text in [
        &view.judgement,
        &view.documentation_task,
        &view.functional_task,
        &view.parameters_task,
        &view.dependencies_task,
    ] {
        ensure!(text.len() <= MAX_TASK * 4, ViewError::Invalid(format!("a task over {MAX_TASK} characters")));
    }

    match view.kind.as_str() {
        "evaluator" => Ok(AgentRole::Evaluator(EvaluatorRole {
            judgement: view.judgement,
            documentation_task: view.documentation_task,
            functional_task: view.functional_task,
            document_passages: view.document_passages,
            dependency_findings: view.dependency_findings,
        })),
        "planner" => Ok(AgentRole::Planner(PlannerRole {
            parameters_task: view.parameters_task,
            dependencies_task: view.dependencies_task,
            summary_characters: view.summary_characters,
        })),
        "chat" => Ok(AgentRole::Chat),
        _ => Err(ViewError::Invalid(format!("no role \"{}\" of an agent", view.kind))),
    }
}

/// Every agent, by its name, with its role.
///
/// Input: `session`, the session of the window, someone logged in.
/// Output: the agents (`[{"id": 2, "name": "Evaluator", "integration":
/// "Ollama", "knowledgeBases": 1, "role": "evaluator"}]`); `ViewError::NotLoggedIn` when nobody is
/// logged in, an error of the database.
///
/// Called by: `ui/src/lib/api/agents.ts`: `agentList`, from `ui/src/views/agents/AgentList.svelte`, when the panel
/// opens and after a change.
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/ai/agent/agent_store.rs`: `read_agents`; `objects/ai/integration/integration_store.rs`:
/// `read_integration`.
#[tauri::command]
pub fn agent_list(session: State<'_, Session>) -> Result<Vec<AgentSummaryView>, ViewError> {
    let opened = session.lock()?;
    let db = database(opened.as_ref())?;
    let mut agents = Vec::new();

    for agent in read_agents(db)? {
        let integration = read_integration(db, agent.integration)?;

        agents.push(AgentSummaryView {
            id: agent.id,
            name: agent.name,
            integration: format!("{} · {}", integration.name, integration.model),
            knowledge_bases: agent.knowledge_bases.len(),
            role: role_name(&agent.role),
        });
    }

    Ok(agents)
}

/// Makes a new agent of the integration `integration` with the role `role`;
/// gives its id.
///
/// Input: `session`, the session of the window, someone logged in; `name`, the
/// name typed; `integration`, the id of an integration; `role`, the role
/// chosen (`"evaluator"`, `"planner"`, `"chat"`).
/// Output: the id of the new agent (`3`); `ViewError::NotLoggedIn` when nobody
/// is logged in, `ViewError::Invalid` for a role too long, the refusals of the
/// agent.
///
/// Called by: `ui/src/lib/api/agents.ts`: `agentCreate`, from `ui/src/views/agents/AgentList.svelte`, "Make the
/// agent".
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/ai/agent/agent_rules.rs`: `new_agent`.
#[tauri::command]
pub fn agent_create(
    session: State<'_, Session>,
    name: String,
    integration: i64,
    role: String,
) -> Result<i64, ViewError> {
    ensure!(name.len() <= MAX_FIELD, ViewError::Invalid("the name is too long".to_owned()));
    ensure!(role.len() <= MAX_FIELD, ViewError::Invalid("the role is too long".to_owned()));

    let opened = session.lock()?;

    Ok(new_agent(database(opened.as_ref())?, &name, integration, &role)?)
}

/// The agent `id` whole, with its role and its parameters, and every
/// integration and knowledge base to choose from.
///
/// Input: `session`, the session of the window, someone logged in; `id`, the id
/// of an agent.
/// Output: the agent with what its editor offers (`{"id": 3, "name": ...,
/// "role": {"kind": "evaluator", ...}, "integrations": [...], "bases":
/// [...]}`); `ViewError::NotLoggedIn` when
/// nobody is logged in, an error of the database.
///
/// Called by: `ui/src/lib/api/agents.ts`: `agentGet`, from `ui/src/views/agents/AgentEditor.svelte`, when an agent
/// is opened and after a save.
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/ai/agent/agent_store.rs`: `read_agent`; `objects/ai/integration/integration_store.rs`:
/// `read_integrations`; `objects/ai/rag/rag_store.rs`: `read_knowledge_bases`.
#[tauri::command]
pub fn agent_get(session: State<'_, Session>, id: i64) -> Result<AgentView, ViewError> {
    let opened = session.lock()?;
    let db = database(opened.as_ref())?;
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
        max_answer_tokens: agent.max_answer_tokens,
        role: role_view(agent.role),
        integrations,
        bases,
    })
}

/// The fields of an agent from the page, checked for their size, as the
/// model.
///
/// Input: `fields`, the fields of the editor of the page.
/// Output: the agent (`Agent { id: 3, .. }`); `ViewError::Invalid` for a
/// field over its size or a role unknown.
///
/// Called by: [`agent_save`].
/// Calls: [`role_of`].
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
        max_answer_tokens: fields.max_answer_tokens,
        role: role_of(fields.role)?,
    })
}

/// Saves the agent `fields.id`.
///
/// Input: `session`, the session of the window, someone logged in; `fields`,
/// the fields of the editor of the page.
/// Output: nothing; `ViewError::NotLoggedIn` when nobody is logged in,
/// `ViewError::Invalid` for a field over its size, the refusals of the agent.
///
/// Called by: `ui/src/lib/api/agents.ts`: `agentSave`, from `ui/src/views/agents/AgentEditor.svelte`, "Save".
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`; [`agent_of`];
/// `objects/ai/agent/agent_rules.rs`: `change_agent`.
#[tauri::command]
pub fn agent_save(session: State<'_, Session>, fields: AgentFields) -> Result<(), ViewError> {
    let agent = agent_of(fields)?;
    let opened = session.lock()?;

    Ok(change_agent(database(opened.as_ref())?, &agent)?)
}

/// The knowledge the agent `id` would give with the question `question`:
/// the test of its bases, its glossary and its limits, before it works on
/// a work unit. Nothing is sent to an AI service.
///
/// Input: `session`, the session of the window, someone logged in; `id`, the id
/// of an agent; `question`, the question typed.
/// Output: the passages it would give (`["The TSF shall ..."]`);
/// `ViewError::NotLoggedIn` when nobody is logged in, an error of the agent or
/// the database.
///
/// Called by: `ui/src/lib/api/agents.ts`: `agentKnowledgeTest`, from `ui/src/views/agents/AgentEditor.svelte`,
/// "Show the knowledge".
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/ai/agent/agent_ai.rs`: `agent_knowledge`.
#[tauri::command]
pub fn agent_knowledge_test(session: State<'_, Session>, id: i64, question: String) -> Result<Vec<String>, ViewError> {
    ensure!(question.len() <= MAX_FIELD, ViewError::Invalid("the question is too long".to_owned()));

    let opened = session.lock()?;

    Ok(agent_knowledge(database(opened.as_ref())?, id, &question)?)
}
