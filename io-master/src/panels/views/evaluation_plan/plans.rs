//! The plans of the panel of the plan, as the page calls them: the list of
//! the plans in planning, a new plan, the data of a plan (its standards,
//! places of documents, verdicts) read and saved, a new place of a
//! document.
//!
//! ```text
//! EvaluationPlan.svelte ──plan_list()─────────────► the plans in planning ──► [PlanSummaryView]
//!                       ──plan_create(name, desc)─► create_plan ──► id
//! PlanForm.svelte       ──plan_get(id)────────────► the plan, the standards, the documents ──► PlanView
//!                       ──plan_save(fields)───────► update_plan
//!                       ──plan_add_document(name)─► add_document_place ──► id
//! ```

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::ai::agent::agent_store::read_agents;
use crate::objects::evaluation::evaluation_plan::evaluation_plan_model::{EvaluationPlan, PlanState};
use crate::objects::evaluation::evaluation_plan::evaluation_plan_rules::planning::{
    add_document_place, create_plan, update_plan,
};
use crate::objects::evaluation::evaluation_plan::evaluation_plan_store::{read_evaluation_plan, read_evaluation_plans};
use crate::objects::evaluation::oscal_standard_document::oscal_standard_document_store::read_oscal_standard_documents;
use crate::objects::evaluation::product_document::product_document_store::read_product_document;
use crate::objects::evaluation::work_unit::work_unit_model::Verdict;
use crate::panels::session::Session;
use crate::panels::views::error::ViewError;
use crate::panels::views::opened::database;

/// A plan in planning, as the list shows it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanSummaryView {
    id: i64,
    name: String,
    work_units: usize,
    blocked: bool,
}

/// A standard of the user, and whether the plan has it.
#[derive(Serialize)]
pub struct StandardChoiceView {
    id: i64,
    title: String,
    chosen: bool,
}

/// A place of a document of the product.
#[derive(Serialize)]
pub struct DocumentView {
    id: i64,
    name: String,
}

/// An agent the plan may name as the agent of its planner.
#[derive(Serialize)]
pub struct AgentChoiceView {
    id: i64,
    name: String,
}

/// The data of a plan, as its form shows it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanView {
    id: i64,
    name: String,
    description: String,
    standards: Vec<StandardChoiceView>,
    documents: Vec<DocumentView>,
    /// "pass", "fail", "inconclusive".
    verdicts: Vec<String>,
    blocked: bool,
    /// The agent of the planner, which fills the work units ("Fill with AI").
    ai_agent: Option<i64>,
    /// Every agent of the user, to choose the agent of the planner.
    agents: Vec<AgentChoiceView>,
}

/// The name of a verdict on the page.
///
/// Input: `verdict`, a verdict.
/// Output: its name on the page (`"pass"`).
///
/// Called by: [`plan_get`].
fn verdict_name(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Pass => "pass",
        Verdict::Fail => "fail",
        Verdict::Inconclusive => "inconclusive",
    }
}

/// The verdict of its name on the page; a name the page never sends is
/// refused.
///
/// Input: `name`, the name of a verdict the page sends (`"pass"`).
/// Output: the verdict (`Verdict::Pass`); `ViewError::Invalid` for another
/// name.
///
/// Called by: [`plan_save`].
fn verdict_of(name: &str) -> Result<Verdict, ViewError> {
    match name {
        "pass" => Ok(Verdict::Pass),
        "fail" => Ok(Verdict::Fail),
        "inconclusive" => Ok(Verdict::Inconclusive),
        _ => Err(ViewError::Invalid(format!("no verdict \"{name}\""))),
    }
}

/// The plans in planning, the newest first.
///
/// Input: `session`, the session of the window, someone logged in.
/// Output: the summaries (`[{"id": 1, "name": "CC EAL4+ router", ...}]`);
/// `ViewError::NotLoggedIn` when nobody is logged in, an error of the database.
///
/// Called by: `ui/src/lib/api/evaluation_plan.ts`: `planList`, from
/// `ui/src/views/evaluation_plan/EvaluationPlan.svelte`, when the panel opens and after a change.
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/evaluation/evaluation_plan/evaluation_plan_store.rs`: `read_evaluation_plans`.
#[tauri::command]
pub fn plan_list(session: State<'_, Session>) -> Result<Vec<PlanSummaryView>, ViewError> {
    let opened = session.lock()?;
    let mut plans = Vec::new();

    for plan in read_evaluation_plans(database(opened.as_ref())?)? {
        if plan.state == PlanState::Plan {
            plans.push(PlanSummaryView {
                id: plan.id,
                name: plan.name,
                work_units: plan.work_units.len(),
                blocked: plan.blocked,
            });
        }
    }

    Ok(plans)
}

/// Makes a new plan in planning. Gives its id.
///
/// Input: `session`, the session of the window, someone logged in; `name`,
/// `description`, as typed.
/// Output: the id of the new plan (`1`); `ViewError::NotLoggedIn` when nobody
/// is logged in, the refusals of the plan.
///
/// Called by: `ui/src/lib/api/evaluation_plan.ts`: `planCreate`, from
/// `ui/src/views/evaluation_plan/EvaluationPlan.svelte`, "New plan".
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/evaluation/evaluation_plan/evaluation_plan_rules/planning.rs`: `create_plan`.
#[tauri::command]
pub fn plan_create(session: State<'_, Session>, name: String, description: String) -> Result<i64, ViewError> {
    let opened = session.lock()?;

    Ok(create_plan(database(opened.as_ref())?, &name, &description)?)
}

/// The view of the plan `plan`: its data, every standard of the user with
/// whether the plan has it, the names of its places of documents.
///
/// Input: `db`, the open database; `plan`, a plan.
/// Output: its view for the form; an error of the database.
///
/// Called by: [`plan_get`].
fn plan_view(db: &UserDatabase, plan: EvaluationPlan) -> Result<PlanView, ViewError> {
    let mut standards = Vec::new();

    for standard in read_oscal_standard_documents(db)? {
        let chosen = plan.oscal_documents.contains(&standard.id);

        standards.push(StandardChoiceView { id: standard.id, title: standard.title, chosen });
    }

    let mut documents = Vec::with_capacity(plan.product_documents.len());

    for id in &plan.product_documents {
        documents.push(DocumentView { id: *id, name: read_product_document(db, *id)?.name });
    }

    let mut verdicts = Vec::with_capacity(plan.possible_verdicts.len());
    let mut agents = Vec::new();

    for agent in read_agents(db)? {
        agents.push(AgentChoiceView { id: agent.id, name: agent.name });
    }

    for verdict in &plan.possible_verdicts {
        verdicts.push(verdict_name(*verdict).to_owned());
    }

    Ok(PlanView {
        id: plan.id,
        name: plan.name,
        description: plan.description,
        standards,
        documents,
        verdicts,
        blocked: plan.blocked,
        ai_agent: plan.ai_agent,
        agents,
    })
}

/// The data of the plan `id`, for its form.
///
/// Input: `session`, the session of the window, someone logged in; `id`, the id
/// of a plan.
/// Output: the view of the plan; `ViewError::NotLoggedIn` when nobody is logged
/// in, an error of the database.
///
/// Called by: `ui/src/lib/api/evaluation_plan.ts`: `planGet`, from `ui/src/views/evaluation_plan/PlanForm.svelte`,
/// when a plan is chosen and after a change.
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/evaluation/evaluation_plan/evaluation_plan_store.rs`: `read_evaluation_plan`; [`plan_view`].
#[tauri::command]
pub fn plan_get(session: State<'_, Session>, id: i64) -> Result<PlanView, ViewError> {
    let opened = session.lock()?;
    let db = database(opened.as_ref())?;

    plan_view(db, read_evaluation_plan(db, id)?)
}

/// The data of a plan the planner changed, as the page sends it from the
/// form of the plan.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanFields {
    /// The plan.
    id: i64,
    name: String,
    description: String,
    /// The standards it names, by their ids.
    standards: Vec<i64>,
    /// The places of the documents of the product, by their ids.
    documents: Vec<i64>,
    /// The verdicts it knows, by their names ("pass").
    verdicts: Vec<String>,
    /// The agent of its planner.
    ai_agent: Option<i64>,
}

/// Saves the data of the plan the planner changed: its name, description,
/// standards, places of documents, verdicts and the agent of its planner.
///
/// Input: `session`, the session of the window, someone logged in; `fields`,
/// the fields of the form.
/// Output: nothing; `ViewError::NotLoggedIn` when nobody is logged in,
/// `ViewError::Invalid` for a verdict the page never sends, the refusals of the
/// plan.
///
/// Called by: `ui/src/lib/api/evaluation_plan.ts`: `planSave`, from `ui/src/views/evaluation_plan/PlanForm.svelte`,
/// "Save".
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/evaluation/evaluation_plan/evaluation_plan_store.rs`: `read_evaluation_plan`;
/// `objects/evaluation/evaluation_plan/evaluation_plan_rules/planning.rs`: `update_plan`.
#[tauri::command]
pub fn plan_save(session: State<'_, Session>, fields: PlanFields) -> Result<(), ViewError> {
    let opened = session.lock()?;
    let db = database(opened.as_ref())?;
    let mut possible_verdicts = Vec::with_capacity(fields.verdicts.len());

    for verdict in &fields.verdicts {
        possible_verdicts.push(verdict_of(verdict)?);
    }

    let plan = EvaluationPlan {
        name: fields.name,
        description: fields.description,
        oscal_documents: fields.standards,
        product_documents: fields.documents,
        possible_verdicts,
        ai_agent: fields.ai_agent,
        ..read_evaluation_plan(db, fields.id)?
    };

    Ok(update_plan(db, &plan)?)
}

/// Reserves a new place of a document of the product (`name`: "ST") in the
/// plan `id`. Gives the id of the place.
///
/// Input: `session`, the session of the window, someone logged in; `id`, the id
/// of a plan in planning; `name`, the name typed.
/// Output: the id of the place (`9`); `ViewError::NotLoggedIn` when nobody is
/// logged in, the refusals of the plan or the name.
///
/// Called by: `ui/src/lib/api/evaluation_plan.ts`: `planAddDocument`, from
/// `ui/src/views/evaluation_plan/PlanForm.svelte`, "Add a document".
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/evaluation/evaluation_plan/evaluation_plan_rules/planning.rs`: `add_document_place`.
#[tauri::command]
pub fn plan_add_document(session: State<'_, Session>, id: i64, name: String) -> Result<i64, ViewError> {
    let opened = session.lock()?;

    Ok(add_document_place(database(opened.as_ref())?, id, &name)?)
}
