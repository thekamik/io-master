//! The plans of the panel of the plan, as the page calls them: the list of
//! the plans in planning, a new plan, the data of a plan (its standards,
//! places of documents, verdicts) read and saved, a new place of a
//! document.
//!
//! ```text
//! EvaluationPlan.svelte ──plan_list()─────────────► the plans in planning ──► [PlanSummaryView]
//!                       ──plan_create(name, desc)─► create_plan ──► id
//! PlanForm.svelte       ──plan_get(id)────────────► the plan, the standards, the documents ──► PlanView
//!                       ──plan_save(...)──────────► update_plan
//!                       ──plan_add_document(name)─► add_document_place ──► id
//! ```

use serde::Serialize;
use tauri::State;

use crate::db::database::Database;
use crate::db::evaluation::evaluation_plan::{read_evaluation_plan, read_evaluation_plans};
use crate::db::evaluation::oscal_standard_document::read_oscal_standard_documents;
use crate::db::evaluation::product_document::read_product_document;
use crate::model::evaluation::evaluation_plan::{EvaluationPlan, PlanState};
use crate::model::evaluation::work_unit::Verdict;
use crate::panels::logic::evaluation_plan::plans::{add_document_place, create_plan, update_plan};
use crate::panels::views::error::ViewError;
use crate::panels::views::opened::database;
use crate::session::state::Session;

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

/// The data of a plan, as its form shows it.
#[derive(Serialize)]
pub struct PlanView {
    id: i64,
    name: String,
    description: String,
    standards: Vec<StandardChoiceView>,
    documents: Vec<DocumentView>,
    /// "pass", "fail", "inconclusive".
    verdicts: Vec<String>,
    blocked: bool,
}

/// The name of a verdict on the page.
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
/// Called by: `EvaluationPlan.svelte`, when the panel opens and after a change.
/// Calls: `read_evaluation_plans`.
#[tauri::command]
pub fn plan_list(session: State<'_, Session>) -> Result<Vec<PlanSummaryView>, ViewError> {
    let opened = session.lock()?;
    let mut plans = Vec::new();

    for plan in read_evaluation_plans(database(&opened)?)? {
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
/// Called by: `EvaluationPlan.svelte`, "New plan".
/// Calls: `create_plan`.
#[tauri::command]
pub fn plan_create(session: State<'_, Session>, name: String, description: String) -> Result<i64, ViewError> {
    let opened = session.lock()?;

    Ok(create_plan(database(&opened)?, &name, &description)?)
}

/// The view of the plan `plan`: its data, every standard of the user with
/// whether the plan has it, the names of its places of documents.
///
/// Called by: [`plan_get`].
fn plan_view(db: &Database, plan: EvaluationPlan) -> Result<PlanView, ViewError> {
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
    })
}

/// The data of the plan `id`, for its form.
///
/// Called by: `PlanForm.svelte`, when a plan is chosen and after a change.
/// Calls: `read_evaluation_plan`, [`plan_view`].
#[tauri::command]
pub fn plan_get(session: State<'_, Session>, id: i64) -> Result<PlanView, ViewError> {
    let opened = session.lock()?;
    let db = database(&opened)?;

    plan_view(db, read_evaluation_plan(db, id)?)
}

/// Saves the data of the plan `id` the planner changed: its name,
/// description, standards, places of documents and verdicts.
///
/// Called by: `PlanForm.svelte`, "Save".
/// Calls: `read_evaluation_plan`, `update_plan`.
#[tauri::command]
#[allow(
    clippy::too_many_arguments,
    reason = "one argument per field of the form of a plan, named as the page sends them"
)]
pub fn plan_save(
    session: State<'_, Session>,
    id: i64,
    name: String,
    description: String,
    standards: Vec<i64>,
    documents: Vec<i64>,
    verdicts: Vec<String>,
) -> Result<(), ViewError> {
    let opened = session.lock()?;
    let db = database(&opened)?;
    let mut possible_verdicts = Vec::with_capacity(verdicts.len());

    for verdict in &verdicts {
        possible_verdicts.push(verdict_of(verdict)?);
    }

    let plan = EvaluationPlan {
        name,
        description,
        oscal_documents: standards,
        product_documents: documents,
        possible_verdicts,
        ..read_evaluation_plan(db, id)?
    };

    Ok(update_plan(db, &plan)?)
}

/// Reserves a new place of a document of the product (`name`: "ST") in the
/// plan `id`. Gives the id of the place.
///
/// Called by: `PlanForm.svelte`, "Add a document".
/// Calls: `add_document_place`.
#[tauri::command]
pub fn plan_add_document(session: State<'_, Session>, id: i64, name: String) -> Result<i64, ViewError> {
    let opened = session.lock()?;

    Ok(add_document_place(database(&opened)?, id, &name)?)
}
