//! The work units of the panel of the plan, as the page calls them: the list
//! of the work units of a plan, the controls of a standard the plan may
//! take, a work unit added (one, or all), removed, read and given its
//! parameters, a place of a proof of the laboratory added.
//!
//! ```text
//! WorkUnits.svelte    ──work_unit_list(plan)───────────────► [WorkUnitRowView]
//!                     ──work_unit_controls(plan, standard)─► available_controls ──► [ControlView]
//!                     ──work_unit_add / _add_all / _remove─► add_work_unit, add_all_work_units, remove_work_unit
//! WorkUnitForm.svelte ──work_unit_get(unit)────────────────► WorkUnitView
//!                     ──work_unit_save(plan, unit, ...)────► update_work_unit_parameters
//!                     ──work_unit_add_proof(plan, unit, name)► add_proof_place
//! ```

use serde::Serialize;
use tauri::State;

use crate::db::database::Database;
use crate::db::evaluation::lab_proof::read_lab_proof;
use crate::db::evaluation::oscal_standard_document::read_oscal_standard_document;
use crate::db::evaluation::work_unit::{read_work_unit, read_work_units};
use crate::panels::logic::evaluation_plan::work_units::{
    WorkUnitParameters, add_all_work_units, add_proof_place, add_work_unit, available_controls, remove_work_unit,
    update_work_unit_parameters,
};
use crate::panels::views::error::ViewError;
use crate::panels::views::opened::database;
use crate::session::state::Session;

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
    cross_reference: bool,
}

/// The title of the standard `id`.
///
/// Called by: [`work_unit_list`].
fn standard_title(db: &Database, id: i64) -> Result<String, ViewError> {
    Ok(read_oscal_standard_document(db, id)?.title)
}

/// The work units of the plan `plan`, in its order.
///
/// Called by: `WorkUnits.svelte`, when a plan is chosen and after a change.
/// Calls: `read_work_units`, [`standard_title`].
#[tauri::command]
pub fn work_unit_list(session: State<'_, Session>, plan: i64) -> Result<Vec<WorkUnitRowView>, ViewError> {
    let opened = session.lock()?;
    let db = database(&opened)?;
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

        let standard = match standard {
            Some(title) => title,
            None => {
                let title = standard_title(db, unit.reference)?;

                titles.push((unit.reference, title.clone()));
                title
            }
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
/// Called by: `WorkUnits.svelte`, when a standard is chosen to add from.
/// Calls: `available_controls`.
#[tauri::command]
pub fn work_unit_controls(
    session: State<'_, Session>,
    plan: i64,
    standard: i64,
) -> Result<Vec<ControlView>, ViewError> {
    let opened = session.lock()?;
    let mut controls = Vec::new();

    for control in available_controls(database(&opened)?, plan, standard)? {
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
/// Called by: `WorkUnits.svelte`, "Add" at a control.
/// Calls: `add_work_unit`.
#[tauri::command]
pub fn work_unit_add(
    session: State<'_, Session>,
    plan: i64,
    standard: i64,
    oscal_key: String,
) -> Result<i64, ViewError> {
    let opened = session.lock()?;

    Ok(add_work_unit(database(&opened)?, plan, standard, &oscal_key)?)
}

/// Adds every control of the standard `standard` the plan `plan` does not
/// hold yet. Gives how many were added.
///
/// Called by: `WorkUnits.svelte`, "Add all".
/// Calls: `add_all_work_units`.
#[tauri::command(async)]
pub fn work_unit_add_all(session: State<'_, Session>, plan: i64, standard: i64) -> Result<usize, ViewError> {
    let opened = session.lock()?;

    Ok(add_all_work_units(database(&opened)?, plan, standard)?)
}

/// Removes the work unit `unit` from the plan `plan`.
///
/// Called by: `WorkUnits.svelte`, "Remove" at a work unit.
/// Calls: `remove_work_unit`.
#[tauri::command]
pub fn work_unit_remove(session: State<'_, Session>, plan: i64, unit: i64) -> Result<(), ViewError> {
    let opened = session.lock()?;

    Ok(remove_work_unit(database(&opened)?, plan, unit)?)
}

/// The work unit `unit`, for its form, with the names of its proofs.
///
/// Called by: `WorkUnitForm.svelte`, when a work unit is chosen.
/// Calls: `read_work_unit`, `read_lab_proof`.
#[tauri::command]
pub fn work_unit_get(session: State<'_, Session>, unit: i64) -> Result<WorkUnitView, ViewError> {
    let opened = session.lock()?;
    let db = database(&opened)?;
    let stored = read_work_unit(db, unit)?;
    let mut proofs = Vec::new();

    for id in stored.input_functional.unwrap_or_default() {
        proofs.push(ProofView { id, name: read_lab_proof(db, id)?.name });
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
        cross_reference: stored.cross_reference,
    })
}

/// Saves the parameters of the work unit `unit` of the plan `plan`.
///
/// Called by: `WorkUnitForm.svelte`, "Save".
/// Calls: `update_work_unit_parameters`.
#[tauri::command]
#[allow(
    clippy::too_many_arguments,
    reason = "one argument per field of the form of a work unit, named as the page sends them"
)]
pub fn work_unit_save(
    session: State<'_, Session>,
    plan: i64,
    unit: i64,
    dependencies: Vec<i64>,
    documents: Vec<i64>,
    proofs: Vec<i64>,
    ai_agent: Option<i64>,
    guidance: String,
) -> Result<(), ViewError> {
    let opened = session.lock()?;
    let parameters = WorkUnitParameters {
        dependencies,
        input_product_documents: documents,
        input_functional: proofs,
        ai_agent,
        guidance,
    };

    Ok(update_work_unit_parameters(database(&opened)?, plan, unit, &parameters)?)
}

/// Reserves a place of a proof of the laboratory (`name`) for the work unit
/// `unit` of the plan `plan`. Gives the id of the place.
///
/// Called by: `WorkUnitForm.svelte`, "Add a proof".
/// Calls: `add_proof_place`.
#[tauri::command]
pub fn work_unit_add_proof(session: State<'_, Session>, plan: i64, unit: i64, name: String) -> Result<i64, ViewError> {
    let opened = session.lock()?;

    Ok(add_proof_place(database(&opened)?, plan, unit, &name)?)
}
