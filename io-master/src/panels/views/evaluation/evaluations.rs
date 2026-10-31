//! The evaluations of the panel of the evaluation, as the page calls them:
//! the list, a new evaluation on a copy of a plan, one evaluation with its
//! inputs, its agent changed.
//!
//! ```text
//! Evaluations.svelte      ──evaluation_list()──────────────► the evaluations and reports ──► [EvaluationSummaryView]
//!                         ──evaluation_create(plan, ...)───► new_evaluation ──► id
//! EvaluationEditor.svelte ──evaluation_get(id)─────────────► the evaluation, its inputs, the agents ──► EvaluationView
//!                         ──evaluation_set_agent(id, agent)► change_agent
//! ```

use serde::Serialize;
use tauri::State;

use crate::db::models::ai::agent::read_agents;
use crate::db::models::evaluation::evaluation_plan::{read_evaluation_plan, read_evaluation_plans};
use crate::db::storage::user_database::UserDatabase;
use crate::model::evaluation::evaluation_plan::{EvaluationPlan, PlanState};
use crate::panels::logic::evaluation::evaluations::{change_agent, new_evaluation};
use crate::panels::logic::evaluation::inputs::inputs;
use crate::panels::views::error::ViewError;
use crate::panels::views::opened::database;
use crate::session::state::Session;

/// The longest product, client or evaluator from the page, in bytes (the
/// logic checks the characters).
const MAX_FIELD: usize = 1024;

/// An evaluation, as the list shows it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationSummaryView {
    id: i64,
    /// The name of the plan it was copied from.
    name: String,
    product: String,
    client: String,
    evaluator: String,
    /// Seconds since 1970 (UTC).
    started: Option<i64>,
    /// "evaluation" or "report".
    state: &'static str,
    work_units: usize,
}

/// An agent to choose.
#[derive(Serialize)]
pub struct AgentChoiceView {
    id: i64,
    name: String,
}

/// A document of the product of an evaluation, without its file.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentInputView {
    id: i64,
    name: String,
    /// `None` while it is not given.
    version: Option<String>,
    /// The size of its file, 0 while it is not given.
    bytes: usize,
    needed_by: usize,
}

/// A proof of the laboratory of an evaluation.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofInputView {
    id: i64,
    name: String,
    /// The text and the record of its test; `None` while it is not given.
    proof: Option<String>,
    performed: Option<i64>,
    tester: Option<String>,
    equipment: Option<String>,
    needed_by: usize,
}

/// An evaluation whole, with its inputs and the agents to choose.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationView {
    summary: EvaluationSummaryView,
    /// The agent of the work units without one of their own.
    ai_agent: Option<i64>,
    agents: Vec<AgentChoiceView>,
    documents: Vec<DocumentInputView>,
    proofs: Vec<ProofInputView>,
}

/// The name of a state on the page.
///
/// Called by: [`summary_of`].
fn state_name(state: PlanState) -> &'static str {
    match state {
        PlanState::Plan => "plan",
        PlanState::Evaluation => "evaluation",
        PlanState::Report => "report",
    }
}

/// The summary of the evaluation `plan`.
///
/// Called by: [`evaluation_list`], [`evaluation_get`].
/// Calls: [`state_name`].
fn summary_of(plan: EvaluationPlan) -> EvaluationSummaryView {
    EvaluationSummaryView {
        id: plan.id,
        name: plan.name,
        product: plan.product,
        client: plan.client,
        evaluator: plan.evaluator,
        started: plan.started,
        state: state_name(plan.state),
        work_units: plan.work_units.len(),
    }
}

/// Every evaluation and report, the newest first.
///
/// Called by: `Evaluations.svelte`, when the panel opens and after a change.
/// Calls: `read_evaluation_plans`, [`summary_of`].
#[tauri::command]
pub fn evaluation_list(session: State<'_, Session>) -> Result<Vec<EvaluationSummaryView>, ViewError> {
    let opened = session.lock()?;
    let mut evaluations = Vec::new();

    for plan in read_evaluation_plans(database(&opened)?)? {
        if plan.state != PlanState::Plan {
            evaluations.push(summary_of(plan));
        }
    }

    Ok(evaluations)
}

/// Begins the evaluation of `product` for `client` by `evaluator` on a copy
/// of the plan `plan`, with the agent `agent`; gives its id.
///
/// Called by: `Evaluations.svelte`, "Begin the evaluation".
/// Calls: `new_evaluation`.
#[tauri::command]
pub fn evaluation_create(
    session: State<'_, Session>,
    plan: i64,
    product: String,
    client: String,
    evaluator: String,
    agent: i64,
) -> Result<i64, ViewError> {
    ensure!(
        product.len() + client.len() + evaluator.len() <= MAX_FIELD,
        ViewError::Invalid("the identification is too long".to_owned())
    );

    let opened = session.lock()?;

    Ok(new_evaluation(database(&opened)?, plan, [&product, &client, &evaluator], agent)?)
}

/// The places of input of the evaluation `id`, as the page shows them.
///
/// Called by: [`evaluation_get`].
/// Calls: `inputs`.
fn input_views(db: &UserDatabase, id: i64) -> Result<(Vec<DocumentInputView>, Vec<ProofInputView>), ViewError> {
    let found = inputs(db, id)?;
    let mut documents = Vec::with_capacity(found.documents.len());
    let mut proofs = Vec::with_capacity(found.proofs.len());

    for needed in found.documents {
        let document = needed.input;

        documents.push(DocumentInputView {
            id: document.id,
            name: document.name,
            version: document.version,
            bytes: document.content.map_or(0, |content| content.len()),
            needed_by: needed.work_units,
        });
    }

    for needed in found.proofs {
        let proof = needed.input;

        proofs.push(ProofInputView {
            id: proof.id,
            name: proof.name,
            proof: proof.proof,
            performed: proof.performed,
            tester: proof.tester,
            equipment: proof.equipment,
            needed_by: needed.work_units,
        });
    }

    Ok((documents, proofs))
}

/// The evaluation `id` whole: its identification, its agent, its inputs and
/// the agents to choose.
///
/// Called by: `EvaluationEditor.svelte`, when an evaluation is opened and
/// after a change.
/// Calls: `read_evaluation_plan`, [`input_views`], `read_agents`,
/// [`summary_of`].
#[tauri::command]
pub fn evaluation_get(session: State<'_, Session>, id: i64) -> Result<EvaluationView, ViewError> {
    let opened = session.lock()?;
    let db = database(&opened)?;
    let evaluation = read_evaluation_plan(db, id)?;

    ensure!(evaluation.state != PlanState::Plan, ViewError::Invalid("a plan in planning is no evaluation".to_owned()));

    let (documents, proofs) = input_views(db, id)?;
    let mut agents = Vec::new();

    for agent in read_agents(db)? {
        agents.push(AgentChoiceView { id: agent.id, name: agent.name });
    }

    Ok(EvaluationView { ai_agent: evaluation.ai_agent, summary: summary_of(evaluation), agents, documents, proofs })
}

/// Gives the evaluation `id` the agent `agent` for its work units without
/// one of their own.
///
/// Called by: `EvaluationEditor.svelte`, the choice of the agent.
/// Calls: `change_agent`.
#[tauri::command]
pub fn evaluation_set_agent(session: State<'_, Session>, id: i64, agent: i64) -> Result<(), ViewError> {
    let opened = session.lock()?;

    Ok(change_agent(database(&opened)?, id, agent)?)
}
