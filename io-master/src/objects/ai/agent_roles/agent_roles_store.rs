//! The roles of the agents (`AgentRole`), beside the agents: the name of the
//! role in the column `role` of `ai_agents` (written by
//! `objects/ai/agent/agent_store.rs`), the parameters of an evaluator or a
//! planner in a table of their own, one row per agent. They are written and
//! read inside the transaction of the agent, never alone.
//!
//! ```text
//! ai_agents.role        'evaluator', 'planner', 'chat'
//! ai_agent_evaluators   (agent, judgement, documentation_task, functional_task, document_passages,
//!                        dependency_findings, task_template, documentation_question, functional_question,
//!                        dependency_template, document_template, document_passage_template, proof_template,
//!                        knowledge_search, document_search)
//! ai_agent_planners     (agent, parameters_task, dependencies_task, summary_characters, parameters_question,
//!                        dependencies_question, summary_template, knowledge_search, default_proof_name)
//! ```

use rusqlite::{Connection, named_params};

use crate::infrastructure::storage::error::DbError;
use crate::objects::ai::agent_roles::agent_roles_model::{AgentRole, EvaluatorRole, PlannerRole};

/// The longest text of a role (a task, the judgement, a template), in
/// characters.
pub const MAX_TASK: usize = 20_000;

/// The most passages of the documents given with a question, as the most a
/// search gives (`objects/ai/rag/rag_store.rs`: `MAX_FOUND`).
pub const MAX_DOCUMENT_PASSAGES: u32 = 50;

/// The most characters of the findings of a dependency given with a question.
pub const MAX_DEPENDENCY_FINDINGS: u32 = 20_000;

/// The most characters of the line about another work unit in the question of
/// the dependencies.
pub const MAX_SUMMARY_CHARACTERS: u32 = 2000;

/// The tables, made when they are missing (`objects/schema.rs`, `USER_TABLES`).
pub const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS ai_agent_evaluators (
        agent               INTEGER PRIMARY KEY REFERENCES ai_agents(id),
        judgement           TEXT NOT NULL,
        documentation_task  TEXT NOT NULL,
        functional_task     TEXT NOT NULL,
        document_passages   INTEGER NOT NULL,
        dependency_findings INTEGER NOT NULL,
        task_template             TEXT NOT NULL,
        documentation_question    TEXT NOT NULL,
        functional_question       TEXT NOT NULL,
        dependency_template       TEXT NOT NULL,
        document_template         TEXT NOT NULL,
        document_passage_template TEXT NOT NULL,
        proof_template            TEXT NOT NULL,
        knowledge_search          TEXT NOT NULL,
        document_search           TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS ai_agent_planners (
        agent              INTEGER PRIMARY KEY REFERENCES ai_agents(id),
        parameters_task    TEXT NOT NULL,
        dependencies_task  TEXT NOT NULL,
        summary_characters INTEGER NOT NULL,
        parameters_question   TEXT NOT NULL,
        dependencies_question TEXT NOT NULL,
        summary_template      TEXT NOT NULL,
        knowledge_search      TEXT NOT NULL,
        default_proof_name    TEXT NOT NULL
    );
";

/// The name of a role as the column `role` of `ai_agents` keeps it:
/// `AgentRole::Chat` is "chat".
///
/// Called by: `objects/ai/agent/agent_store.rs`: `create_agent`, `update_agent`;
/// `objects/ai/agent/agent_rules.rs`: `change_agent`; `panels/views/agents/agent.rs`: `agent_list`,
/// `role_view`.
pub fn role_name(role: &AgentRole) -> &'static str {
    match role {
        AgentRole::Evaluator(_) => "evaluator",
        AgentRole::Planner(_) => "planner",
        AgentRole::Chat => "chat",
    }
}

/// Refuses a role whose parameters are over the limits of the program.
///
/// Input: `role`, the role of an agent to be stored.
/// Output: nothing when it is good; otherwise `DbError::Inconsistent` naming
/// what is wrong ("a text of a role over 20000 characters").
///
/// Called by: `objects/ai/agent/agent_store.rs`: `check_agent`.
pub fn check_agent_role(role: &AgentRole) -> Result<(), DbError> {
    let (texts, numbers_good) = match role {
        AgentRole::Evaluator(evaluator) => (
            vec![
                &evaluator.judgement,
                &evaluator.documentation_task,
                &evaluator.functional_task,
                &evaluator.task_template,
                &evaluator.documentation_question,
                &evaluator.functional_question,
                &evaluator.dependency_template,
                &evaluator.document_template,
                &evaluator.document_passage_template,
                &evaluator.proof_template,
                &evaluator.knowledge_search,
                &evaluator.document_search,
            ],
            (1..=MAX_DOCUMENT_PASSAGES).contains(&evaluator.document_passages)
                && evaluator.dependency_findings <= MAX_DEPENDENCY_FINDINGS,
        ),
        AgentRole::Planner(planner) => (
            vec![
                &planner.parameters_task,
                &planner.dependencies_task,
                &planner.parameters_question,
                &planner.dependencies_question,
                &planner.summary_template,
                &planner.knowledge_search,
                &planner.default_proof_name,
            ],
            (1..=MAX_SUMMARY_CHARACTERS).contains(&planner.summary_characters),
        ),
        AgentRole::Chat => (Vec::new(), true),
    };

    ensure!(numbers_good, DbError::Inconsistent("a limit of a role out of its range".to_owned()));

    for text in texts {
        ensure!(
            text.chars().count() <= MAX_TASK,
            DbError::Inconsistent(format!("a text of a role over {MAX_TASK} characters"))
        );
    }

    Ok(())
}

/// Writes the parameters of the role of the agent `agent` anew; a chat has
/// none. Run inside the transaction of the agent.
///
/// Input: `connection`, the open database inside a transaction; `agent`, the
/// id of the agent; `role`, its role, checked by [`check_agent_role`].
/// Output: nothing; an error of SQLite when the agent is not stored.
///
/// Called by: `objects/ai/agent/agent_store.rs`: `create_agent`, `update_agent`.
pub fn write_agent_role(connection: &Connection, agent: i64, role: &AgentRole) -> Result<(), DbError> {
    connection.execute("DELETE FROM ai_agent_evaluators WHERE agent = :agent", named_params! { ":agent": agent })?;
    connection.execute("DELETE FROM ai_agent_planners WHERE agent = :agent", named_params! { ":agent": agent })?;

    match role {
        AgentRole::Evaluator(evaluator) => {
            connection.execute(
                "INSERT INTO ai_agent_evaluators
                     (agent, judgement, documentation_task, functional_task, document_passages, dependency_findings,
                      task_template, documentation_question, functional_question, dependency_template,
                      document_template, document_passage_template, proof_template, knowledge_search,
                      document_search)
                 VALUES (:agent, :judgement, :documentation, :functional, :passages, :findings, :task_template,
                         :documentation_question, :functional_question, :dependency_template, :document_template,
                         :document_passage_template, :proof_template, :knowledge_search, :document_search)",
                named_params! {
                    ":agent": agent,
                    ":judgement": evaluator.judgement,
                    ":documentation": evaluator.documentation_task,
                    ":functional": evaluator.functional_task,
                    ":passages": evaluator.document_passages,
                    ":findings": evaluator.dependency_findings,
                    ":task_template": evaluator.task_template,
                    ":documentation_question": evaluator.documentation_question,
                    ":functional_question": evaluator.functional_question,
                    ":dependency_template": evaluator.dependency_template,
                    ":document_template": evaluator.document_template,
                    ":document_passage_template": evaluator.document_passage_template,
                    ":proof_template": evaluator.proof_template,
                    ":knowledge_search": evaluator.knowledge_search,
                    ":document_search": evaluator.document_search,
                },
            )?;
        }
        AgentRole::Planner(planner) => {
            connection.execute(
                "INSERT INTO ai_agent_planners
                     (agent, parameters_task, dependencies_task, summary_characters, parameters_question,
                      dependencies_question, summary_template, knowledge_search, default_proof_name)
                 VALUES (:agent, :parameters, :dependencies, :summary, :parameters_question, :dependencies_question,
                         :summary_template, :knowledge_search, :default_proof_name)",
                named_params! {
                    ":agent": agent,
                    ":parameters": planner.parameters_task,
                    ":dependencies": planner.dependencies_task,
                    ":summary": planner.summary_characters,
                    ":parameters_question": planner.parameters_question,
                    ":dependencies_question": planner.dependencies_question,
                    ":summary_template": planner.summary_template,
                    ":knowledge_search": planner.knowledge_search,
                    ":default_proof_name": planner.default_proof_name,
                },
            )?;
        }
        AgentRole::Chat => {}
    }

    Ok(())
}

/// Reads the parameters of the evaluator `agent`.
///
/// Input: `connection`, the open database; `agent`, the id of an agent of the
/// role evaluator.
/// Output: its parameters; `DbError::NoRow` when their row is missing.
///
/// Called by: [`read_agent_role`].
fn read_evaluator(connection: &Connection, agent: i64) -> Result<EvaluatorRole, DbError> {
    let mut statement = connection.prepare(
        "SELECT judgement, documentation_task, functional_task, document_passages, dependency_findings,
                task_template, documentation_question, functional_question, dependency_template, document_template,
                document_passage_template, proof_template, knowledge_search, document_search
         FROM ai_agent_evaluators WHERE agent = :agent",
    )?;
    let mut rows = statement.query(named_params! { ":agent": agent })?;
    let Some(row) = rows.next()? else {
        return Err(DbError::NoRow { table: "ai_agent_evaluators", id: agent });
    };

    Ok(EvaluatorRole {
        judgement: row.get("judgement")?,
        documentation_task: row.get("documentation_task")?,
        functional_task: row.get("functional_task")?,
        document_passages: row.get("document_passages")?,
        dependency_findings: row.get("dependency_findings")?,
        task_template: row.get("task_template")?,
        documentation_question: row.get("documentation_question")?,
        functional_question: row.get("functional_question")?,
        dependency_template: row.get("dependency_template")?,
        document_template: row.get("document_template")?,
        document_passage_template: row.get("document_passage_template")?,
        proof_template: row.get("proof_template")?,
        knowledge_search: row.get("knowledge_search")?,
        document_search: row.get("document_search")?,
    })
}

/// Reads the parameters of the planner `agent`.
///
/// Input: `connection`, the open database; `agent`, the id of an agent of the
/// role planner.
/// Output: its parameters; `DbError::NoRow` when their row is missing.
///
/// Called by: [`read_agent_role`].
fn read_planner(connection: &Connection, agent: i64) -> Result<PlannerRole, DbError> {
    let mut statement = connection.prepare(
        "SELECT parameters_task, dependencies_task, summary_characters, parameters_question, dependencies_question,
                summary_template, knowledge_search, default_proof_name
         FROM ai_agent_planners WHERE agent = :agent",
    )?;
    let mut rows = statement.query(named_params! { ":agent": agent })?;
    let Some(row) = rows.next()? else {
        return Err(DbError::NoRow { table: "ai_agent_planners", id: agent });
    };

    Ok(PlannerRole {
        parameters_task: row.get("parameters_task")?,
        dependencies_task: row.get("dependencies_task")?,
        summary_characters: row.get("summary_characters")?,
        parameters_question: row.get("parameters_question")?,
        dependencies_question: row.get("dependencies_question")?,
        summary_template: row.get("summary_template")?,
        knowledge_search: row.get("knowledge_search")?,
        default_proof_name: row.get("default_proof_name")?,
    })
}

/// Reads the role of the agent `agent` named `name` in its row, with its
/// parameters.
///
/// Input: `connection`, the open database; `agent`, the id of a stored agent;
/// `name`, the column `role` of its row ("evaluator").
/// Output: the role (`AgentRole::Evaluator(EvaluatorRole { .. })`);
/// `DbError::Corrupt` for a name the program never writes, `DbError::NoRow`
/// when the row of its parameters is missing.
///
/// Called by: `objects/ai/agent/agent_store.rs`: `read_agent`.
/// Calls: [`read_evaluator`], [`read_planner`].
pub fn read_agent_role(connection: &Connection, agent: i64, name: &str) -> Result<AgentRole, DbError> {
    match name {
        "evaluator" => Ok(AgentRole::Evaluator(read_evaluator(connection, agent)?)),
        "planner" => Ok(AgentRole::Planner(read_planner(connection, agent)?)),
        "chat" => Ok(AgentRole::Chat),
        _ => Err(DbError::Corrupt { table: "ai_agents", column: "role", value: name.to_owned() }),
    }
}
