//! The work units of the panel of the plan, as the page calls them: the list
//! of the work units of a plan, the controls of a standard the plan may
//! take, a work unit added (one, or all), removed, read and given its
//! parameters, a place of a proof of the laboratory added.
//!
//! ```text
//! PlanWorkUnits.svelte  ──work_unit_list(plan)───────────────► [WorkUnitRowView]
//!                       ──work_unit_controls(plan, standard)─► available_controls ──► [ControlView]
//!                       ──work_unit_add / _add_all / _remove─► add_work_unit, add_all_work_units, remove_work_unit
//! WorkUnitEditor.svelte ──work_unit_get(unit)────────────────► WorkUnitView
//!                       ──work_unit_save(fields)─────────────► update_work_unit_parameters
//!                       ──work_unit_add_proof(plan, unit, name)► add_proof_place
//! ```

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::ai::agent::agent_store::read_agents;
use crate::objects::ai::agent_roles::agent_roles_model::AgentRole;
use crate::objects::evaluation::evaluation_plan::evaluation_plan_rules::work_units::{
    add_all_work_units, add_work_unit, available_controls, remove_work_unit,
};
use crate::objects::evaluation::lab_proof::lab_proof_store::read_lab_proof;
use crate::objects::evaluation::oscal_standard_document::oscal_standard_document_store::read_oscal_standard_document;
use crate::objects::evaluation::work_unit::work_unit_rules::parameters::{
    WorkUnitParameters, add_proof_place, update_work_unit_parameters,
};
use crate::objects::evaluation::work_unit::work_unit_store::{read_work_unit, read_work_units};
use crate::panels::session::Session;
use crate::panels::views::error::ViewError;
use crate::panels::views::opened::database;

/// A work unit, as the list of a plan shows it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkUnitRowView {
    id: i64,
    label: String,
    /// The title of its standard.
    standard: String,
    requirement: String,
    /// Whether it needs proofs of the laboratory.
    functional: bool,
    dependencies: Vec<i64>,
    cross_reference: bool,
}

/// A control of a standard the plan may take.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlView {
    oscal_key: String,
    label: String,
    requirement: String,
}

/// A place of a proof of the laboratory.
#[derive(Serialize)]
pub struct ProofView {
    id: i64,
    name: String,
}

/// An agent the work unit may name.
#[derive(Serialize)]
pub struct AgentChoiceView {
    id: i64,
    name: String,
}

/// A work unit, as its form shows it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkUnitView {
    id: i64,
    label: String,
    requirement: String,
    guidance: String,
    dependencies: Vec<i64>,
    documents: Vec<i64>,
    proofs: Vec<ProofView>,
    ai_agent: Option<i64>,
    /// Every agent of the user, to choose the one that works on the unit.
    agents: Vec<AgentChoiceView>,
    cross_reference: bool,
}

/// The title of the standard `id`.
///
/// Called by: [`work_unit_list`].
fn standard_title(db: &UserDatabase, id: i64) -> Result<String, ViewError> {
    Ok(read_oscal_standard_document(db, id)?.title)
}

/// The work units of the plan `plan`, in its order.
///
/// Input: `session`, the session of the window, someone logged in; `plan`, the
/// id of a plan.
/// Output: the rows (`[{"id": 7, "label": "ADV_ARC.1-1", "standard": "CEM",
/// ...}]`); `ViewError::NotLoggedIn` when nobody is logged in, an error of the
/// database.
///
/// Called by: `ui/src/lib/api/evaluation_plan.ts`: `workUnitList`, from
/// `ui/src/views/evaluation_plan/PlanWorkUnits.svelte`, when a plan is chosen and after a change;
/// `ui/src/views/evaluation_plan/WorkUnitEditor.svelte` (the dependencies to choose).
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/evaluation/work_unit/work_unit_store.rs`: `read_work_units`; [`standard_title`].
#[tauri::command]
pub fn work_unit_list(session: State<'_, Session>, plan: i64) -> Result<Vec<WorkUnitRowView>, ViewError> {
    let opened = session.lock()?;
    let db = database(opened.as_ref())?;
    let units = read_work_units(db, plan)?;
    let mut titles: Vec<(i64, String)> = Vec::new();
    let mut rows = Vec::with_capacity(units.len());

    for unit in units {
        let mut standard = None;

        for (id, title) in &titles {
            if *id == unit.reference {
                standard = Some(title.clone());
            }
        }

        let standard = if let Some(title) = standard {
            title
        } else {
            let title = standard_title(db, unit.reference)?;

            titles.push((unit.reference, title.clone()));
            title
        };

        rows.push(WorkUnitRowView {
            id: unit.id,
            label: unit.label,
            standard,
            requirement: unit.requirement,
            functional: unit.input_functional.is_some(),
            dependencies: unit.dependencies,
            cross_reference: unit.cross_reference,
        });
    }

    Ok(rows)
}

/// The controls of the standard `standard` the plan `plan` does not hold
/// yet.
///
/// Input: `session`, the session of the window, someone logged in; `plan`, the
/// id of a plan; `standard`, the id of one of its standards.
/// Output: the controls (`[{"oscalKey": "adv_arc.1-2", "label": "ADV_ARC.1-2"}]`);
/// `ViewError::NotLoggedIn` when nobody is logged in, the refusals of the standard.
///
/// Called by: `ui/src/lib/api/evaluation_plan.ts`: `workUnitControls`, from
/// `ui/src/views/evaluation_plan/PlanWorkUnits.svelte`, when a standard is chosen to add from.
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/evaluation/evaluation_plan/evaluation_plan_rules/work_units.rs`: `available_controls`.
#[tauri::command]
pub fn work_unit_controls(
    session: State<'_, Session>,
    plan: i64,
    standard: i64,
) -> Result<Vec<ControlView>, ViewError> {
    let opened = session.lock()?;
    let mut controls = Vec::new();

    for control in available_controls(database(opened.as_ref())?, plan, standard)? {
        controls.push(ControlView {
            oscal_key: control.oscal_key,
            label: control.label,
            requirement: control.requirement,
        });
    }

    Ok(controls)
}

/// Adds the work unit of the control `oscal_key` of the standard `standard`
/// to the plan `plan`. Gives its id.
///
/// Input: `session`, the session of the window, someone logged in; `plan`, the
/// id of a plan in planning; `standard`, the id of one of its standards;
/// `oscal_key`, the key of a control of it.
/// Output: the id of the new work unit (`14`); `ViewError::NotLoggedIn` when
/// nobody is logged in, the refusals of the plan.
///
/// Called by: `ui/src/lib/api/evaluation_plan.ts`: `workUnitAdd`, from
/// `ui/src/views/evaluation_plan/PlanWorkUnits.svelte`, "Add" at a control.
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/evaluation/evaluation_plan/evaluation_plan_rules/work_units.rs`: `add_work_unit`.
#[tauri::command]
pub fn work_unit_add(
    session: State<'_, Session>,
    plan: i64,
    standard: i64,
    oscal_key: String,
) -> Result<i64, ViewError> {
    let opened = session.lock()?;

    Ok(add_work_unit(database(opened.as_ref())?, plan, standard, &oscal_key)?)
}

/// Adds every control of the standard `standard` the plan `plan` does not
/// hold yet. Gives how many were added.
///
/// Called by: `ui/src/lib/api/evaluation_plan.ts`: `workUnitAddAll`, from
/// `ui/src/views/evaluation_plan/PlanWorkUnits.svelte`, "Add all".
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/evaluation/evaluation_plan/evaluation_plan_rules/work_units.rs`: `add_all_work_units`.
#[tauri::command(async)]
pub fn work_unit_add_all(session: State<'_, Session>, plan: i64, standard: i64) -> Result<usize, ViewError> {
    let opened = session.lock()?;

    Ok(add_all_work_units(database(opened.as_ref())?, plan, standard)?)
}

/// Removes the work unit `unit` from the plan `plan`.
///
/// Called by: `ui/src/lib/api/evaluation_plan.ts`: `workUnitRemove`, from
/// `ui/src/views/evaluation_plan/PlanWorkUnits.svelte`, "Remove" at a work unit.
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/evaluation/evaluation_plan/evaluation_plan_rules/work_units.rs`: `remove_work_unit`.
#[tauri::command]
pub fn work_unit_remove(session: State<'_, Session>, plan: i64, unit: i64) -> Result<(), ViewError> {
    let opened = session.lock()?;

    Ok(remove_work_unit(database(opened.as_ref())?, plan, unit)?)
}

/// The work unit `unit`, for its form, with the names of its proofs and
/// every agent to choose from.
///
/// Input: `session`, the session of the window, someone logged in; `unit`, the
/// id of a work unit.
/// Output: the work unit for its form; `ViewError::NotLoggedIn` when nobody is
/// logged in, an error of the database.
///
/// Called by: `ui/src/lib/api/evaluation_plan.ts`: `workUnitGet`, from
/// `ui/src/views/evaluation_plan/WorkUnitEditor.svelte`, when a work unit is opened.
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/evaluation/work_unit/work_unit_store.rs`: `read_work_unit`;
/// `objects/evaluation/lab_proof/lab_proof_store.rs`: `read_lab_proof`; `objects/ai/agent/agent_store.rs`:
/// `read_agents`.
#[tauri::command]
pub fn work_unit_get(session: State<'_, Session>, unit: i64) -> Result<WorkUnitView, ViewError> {
    let opened = session.lock()?;
    let db = database(opened.as_ref())?;
    let stored = read_work_unit(db, unit)?;
    let mut proofs = Vec::new();
    let mut agents = Vec::new();

    for id in stored.input_functional.unwrap_or_default() {
        proofs.push(ProofView { id, name: read_lab_proof(db, id)?.name });
    }

    // Only the agents of the role of this work: the agent of a work unit assesses it (user, 2026-11-02).
    for agent in read_agents(db)? {
        if matches!(agent.role, AgentRole::Evaluator(_)) {
            agents.push(AgentChoiceView { id: agent.id, name: agent.name });
        }
    }

    Ok(WorkUnitView {
        id: stored.id,
        label: stored.label,
        requirement: stored.requirement,
        guidance: stored.guidance,
        dependencies: stored.dependencies,
        documents: stored.input_product_documents,
        proofs,
        ai_agent: stored.ai_agent,
        agents,
        cross_reference: stored.cross_reference,
    })
}

/// The parameters of a work unit the planner changed, as the page sends
/// them from the form of the work unit.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkUnitFields {
    /// The plan of the work unit.
    plan: i64,
    /// The work unit.
    unit: i64,
    /// The work units of the same plan it depends on.
    dependencies: Vec<i64>,
    /// The places of the documents of the product it needs.
    documents: Vec<i64>,
    /// The places of the proofs of the laboratory it needs.
    proofs: Vec<i64>,
    /// Its own agent.
    ai_agent: Option<i64>,
    guidance: String,
}

/// Saves the parameters of a work unit.
///
/// Input: `session`, the session of the window, someone logged in; `fields`,
/// the fields of the form.
/// Output: nothing; `ViewError::NotLoggedIn` when nobody is logged in,
/// `ViewError::Invalid` for a field over its size, the refusals of the
/// parameters.
///
/// Called by: `ui/src/lib/api/evaluation_plan.ts`: `workUnitSave`, from
/// `ui/src/views/evaluation_plan/WorkUnitEditor.svelte`, "Save".
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/evaluation/work_unit/work_unit_rules/parameters.rs`: `update_work_unit_parameters`.
#[tauri::command]
pub fn work_unit_save(session: State<'_, Session>, fields: WorkUnitFields) -> Result<(), ViewError> {
    let opened = session.lock()?;
    let parameters = WorkUnitParameters {
        dependencies: fields.dependencies,
        input_product_documents: fields.documents,
        input_functional: fields.proofs,
        ai_agent: fields.ai_agent,
        guidance: fields.guidance,
    };

    Ok(update_work_unit_parameters(database(opened.as_ref())?, fields.plan, fields.unit, &parameters)?)
}

/// Reserves a place of a proof of the laboratory (`name`) for the work unit
/// `unit` of the plan `plan`. Gives the id of the place.
///
/// Called by: `ui/src/lib/api/evaluation_plan.ts`: `workUnitAddProof`, from
/// `ui/src/views/evaluation_plan/WorkUnitEditor.svelte`, "Add a proof".
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/evaluation/work_unit/work_unit_rules/parameters.rs`: `add_proof_place`.
#[tauri::command]
pub fn work_unit_add_proof(session: State<'_, Session>, plan: i64, unit: i64, name: String) -> Result<i64, ViewError> {
    let opened = session.lock()?;

    Ok(add_proof_place(database(opened.as_ref())?, plan, unit, &name)?)
}
