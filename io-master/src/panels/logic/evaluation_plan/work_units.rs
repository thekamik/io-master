//! The work units of a plan in planning: added from a control of one of the
//! standards of the plan (one, or all at once), removed, and given their
//! parameters by the planner (dependencies, documents, proofs of the
//! laboratory, agent, guidance). After every change of the work units the
//! cross references of the plan are marked again (`order.rs`).
//!
//! ```text
//! a standard of the plan ──catalog_controls (oscal.rs)──► a control ──► a new work unit (create_work_unit)
//! the planner ──parameters──► update_work_unit_parameters ──► update_work_unit ──► check_order
//! ```
//!
//! A work unit made from a control has no documents, proofs or agent yet:
//! the planner gives them (and later an agent that reads the standard, user
//! 2026-10-29). A work unit with proofs of the laboratory is a functional
//! test; without any (`None`), a test of the documentation.

use crate::db::database::Database;
use crate::db::evaluation::evaluation_plan::read_evaluation_plan;
use crate::db::evaluation::lab_proof::create_lab_proof;
use crate::db::evaluation::oscal_standard_document::read_oscal_standard_document;
use crate::db::evaluation::work_unit::{
    create_work_unit, delete_work_unit, read_work_unit, read_work_units, update_work_unit,
};
use crate::model::evaluation::evaluation_plan::EvaluationPlan;
use crate::model::evaluation::lab_proof::LabProof;
use crate::model::evaluation::work_unit::WorkUnit;
use crate::panels::logic::evaluation_plan::error::PlanError;
use crate::panels::logic::evaluation_plan::order::check_order;
use crate::panels::logic::evaluation_plan::oscal::{CatalogControl, catalog_controls};
use crate::panels::logic::evaluation_plan::plans::check_planning;

/// What the planner gives a work unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkUnitParameters {
    /// The work units of the plan it depends on.
    pub dependencies: Vec<i64>,
    /// The documents of the product (places of the plan) it needs.
    pub input_product_documents: Vec<i64>,
    /// The proofs of the laboratory it needs; empty: a test of the
    /// documentation.
    pub input_functional: Vec<i64>,
    /// The agent that assesses it; `None`: none.
    pub ai_agent: Option<i64>,
    /// The guidance of the standard, as the planner corrected it.
    pub guidance: String,
}

/// The plan `plan`, refused unless it is in planning.
///
/// Called by: every function of this file.
/// Calls: `read_evaluation_plan`, `plans::check_planning`.
fn plan_in_planning(db: &Database, plan: i64) -> Result<EvaluationPlan, PlanError> {
    let stored = read_evaluation_plan(db, plan)?;

    check_planning(&stored)?;

    Ok(stored)
}

/// The controls of the standard `standard`, one of the standards of `plan`.
///
/// Called by: [`add_work_unit`], [`add_all_work_units`].
/// Calls: `read_oscal_standard_document`, `oscal::catalog_controls`.
fn controls_of(db: &Database, plan: &EvaluationPlan, standard: i64) -> Result<Vec<CatalogControl>, PlanError> {
    ensure!(
        plan.oscal_documents.contains(&standard),
        PlanError::Refused("the standard is not one of the plan".to_owned())
    );

    let document = read_oscal_standard_document(db, standard)?;
    let json = serde_json::from_str(&document.doc)
        .map_err(|error| PlanError::Refused(format!("the stored standard is no JSON: {error}")))?;

    Ok(catalog_controls(&json)?)
}

/// A new work unit of the standard `standard` made from its control
/// `control`: no dependencies, documents, proofs or agent yet.
///
/// Called by: [`add_work_unit`], [`add_all_work_units`].
fn unit_of(standard: i64, control: &CatalogControl) -> WorkUnit {
    WorkUnit {
        id: 0,
        reference: standard,
        label: control.label.clone(),
        oscal_key: control.oscal_key.clone(),
        requirement: control.requirement.clone(),
        guidance: control.guidance.clone(),
        dependencies: Vec::new(),
        cross_reference: false,
        input_product_documents: Vec::new(),
        input_functional: None,
        ai_agent: None,
        findings: String::new(),
        analysis: String::new(),
        vulnerability_notes: String::new(),
        notes: String::new(),
        quotes: Vec::new(),
        verdict: None,
        author: None,
    }
}

/// Whether the plan holds the work unit of the control `oscal_key` of the
/// standard `standard` already.
///
/// Called by: [`add_work_unit`], [`add_all_work_units`].
fn holds(units: &[WorkUnit], standard: i64, oscal_key: &str) -> bool {
    for unit in units {
        if unit.reference == standard && unit.oscal_key == oscal_key {
            return true;
        }
    }

    false
}

/// The controls of the standard `standard` of the plan `plan` that the plan
/// does not hold yet, in the order of the standard: what the planner may
/// add.
///
/// Called by: `panels/views/evaluation_plan/work_units.rs` (`work_unit_controls`).
/// Calls: `read_evaluation_plan`, [`controls_of`], `read_work_units`, [`holds`].
pub fn available_controls(db: &Database, plan: i64, standard: i64) -> Result<Vec<CatalogControl>, PlanError> {
    let stored = read_evaluation_plan(db, plan)?;
    let units = read_work_units(db, plan)?;
    let mut available = Vec::new();

    for control in controls_of(db, &stored, standard)? {
        if !holds(&units, standard, &control.oscal_key) {
            available.push(control);
        }
    }

    Ok(available)
}

/// Adds to the plan `plan` the work unit of the control `oscal_key` of its
/// standard `standard`, as its last. Refused when the plan holds it already.
/// Gives its id.
///
/// Called by: `panels/views/evaluation_plan/` (a work unit added).
/// Calls: [`plan_in_planning`], [`controls_of`], [`holds`], [`unit_of`],
/// `create_work_unit`.
pub fn add_work_unit(db: &Database, plan: i64, standard: i64, oscal_key: &str) -> Result<i64, PlanError> {
    let stored = plan_in_planning(db, plan)?;
    let controls = controls_of(db, &stored, standard)?;

    ensure!(
        !holds(&read_work_units(db, plan)?, standard, oscal_key),
        PlanError::Refused(format!("the plan holds {oscal_key} already"))
    );

    for control in &controls {
        if control.oscal_key == oscal_key {
            return Ok(create_work_unit(db, plan, &unit_of(standard, control))?);
        }
    }

    Err(PlanError::Refused(format!("the standard has no control {oscal_key}")))
}

/// Adds to the plan `plan` every control of its standard `standard` it does
/// not hold yet, in the order of the standard ("add all"). Gives how many
/// were added.
///
/// Called by: `panels/views/evaluation_plan/` ("Add all").
/// Calls: [`plan_in_planning`], [`controls_of`], [`holds`], [`unit_of`],
/// `create_work_unit`.
pub fn add_all_work_units(db: &Database, plan: i64, standard: i64) -> Result<usize, PlanError> {
    let stored = plan_in_planning(db, plan)?;
    let controls = controls_of(db, &stored, standard)?;
    let units = read_work_units(db, plan)?;
    let mut added = 0;

    for control in &controls {
        if !holds(&units, standard, &control.oscal_key) {
            create_work_unit(db, plan, &unit_of(standard, control))?;
            added += 1;
        }
    }

    Ok(added)
}

/// Removes the work unit `unit` from the plan `plan` in planning, with the
/// dependencies of the others on it, and marks the cross references again.
///
/// Called by: `panels/views/evaluation_plan/` (a work unit removed).
/// Calls: [`plan_in_planning`], `delete_work_unit`, `order::check_order`.
pub fn remove_work_unit(db: &Database, plan: i64, unit: i64) -> Result<(), PlanError> {
    let stored = plan_in_planning(db, plan)?;

    ensure!(stored.work_units.contains(&unit), PlanError::Refused("the work unit is not one of the plan".to_owned()));

    delete_work_unit(db, unit)?;
    check_order(db, plan)?;

    Ok(())
}

/// Gives the work unit `unit` of the plan `plan` in planning the
/// `parameters` of the planner, and marks the cross references again. An
/// empty list of proofs makes it a test of the documentation (`None`).
///
/// Called by: `panels/views/evaluation_plan/` (the form of a work unit).
/// Calls: [`plan_in_planning`], `read_work_unit`, `update_work_unit`,
/// `order::check_order`.
pub fn update_work_unit_parameters(
    db: &Database,
    plan: i64,
    unit: i64,
    parameters: &WorkUnitParameters,
) -> Result<(), PlanError> {
    let stored = plan_in_planning(db, plan)?;

    ensure!(stored.work_units.contains(&unit), PlanError::Refused("the work unit is not one of the plan".to_owned()));

    let changed = WorkUnit {
        dependencies: parameters.dependencies.clone(),
        input_product_documents: parameters.input_product_documents.clone(),
        input_functional: if parameters.input_functional.is_empty() {
            None
        } else {
            Some(parameters.input_functional.clone())
        },
        ai_agent: parameters.ai_agent,
        guidance: parameters.guidance.clone(),
        ..read_work_unit(db, unit)?
    };

    update_work_unit(db, &changed)?;
    check_order(db, plan)?;

    Ok(())
}

/// Reserves a new place of a proof of the laboratory (`name`: "results of
/// the scan of the ports") for the work unit `unit` of the plan `plan`,
/// which becomes a functional test. Gives the id of the place.
///
/// Called by: `panels/views/evaluation_plan/` (a proof added to a work unit).
/// Calls: [`plan_in_planning`], `create_lab_proof`, `read_work_unit`,
/// `update_work_unit`.
pub fn add_proof_place(db: &Database, plan: i64, unit: i64, name: &str) -> Result<i64, PlanError> {
    let stored = plan_in_planning(db, plan)?;

    ensure!(stored.work_units.contains(&unit), PlanError::Refused("the work unit is not one of the plan".to_owned()));
    ensure!(!name.trim().is_empty(), PlanError::Refused("a proof without a name".to_owned()));

    let place =
        LabProof { id: 0, name: name.trim().to_owned(), proof: None, performed: None, tester: None, equipment: None };
    let id = create_lab_proof(db, &place)?;
    let mut changed = read_work_unit(db, unit)?;

    changed.input_functional.get_or_insert_with(Vec::new).push(id);
    update_work_unit(db, &changed)?;

    Ok(id)
}

#[cfg(test)]
mod tests {
    //! Tests of the work units of a plan: added from the sample of NIST (one,
    //! all), given their parameters, made a functional test, removed.

    use super::*;

    use crate::panels::logic::evaluation_plan::plans::{add_document_place, create_plan, update_plan};
    use crate::panels::logic::evaluation_plan::standards::import_standard;
    use crate::test_support::database::database;

    const NIST_SAMPLE: &str = include_str!("../../../../oscal/examples/basic-catalog.json");

    /// Whether `controls` hold the control `oscal_key`.
    fn holds_key(controls: &[CatalogControl], oscal_key: &str) -> bool {
        controls.iter().any(|control| control.oscal_key == oscal_key)
    }

    /// A plan with the sample of NIST as its standard.
    fn plan_with_standard(db: &Database) -> (i64, i64) {
        let standard = import_standard(db, NIST_SAMPLE).unwrap();
        let plan = create_plan(db, "Sample", "").unwrap();
        let stored = read_evaluation_plan(db, plan).unwrap();

        update_plan(db, &EvaluationPlan { oscal_documents: vec![standard], ..stored }).unwrap();

        (plan, standard)
    }

    #[test]
    fn a_control_is_added_once_and_all_the_others_at_once() {
        let (_directory, db) = database();
        let (plan, standard) = plan_with_standard(&db);
        let first = add_work_unit(&db, plan, standard, "s1.1.1").unwrap();

        assert_eq!(read_work_unit(&db, first).unwrap().label, "1.1.1");
        assert!(!holds_key(&available_controls(&db, plan, standard).unwrap(), "s1.1.1"));
        assert!(matches!(add_work_unit(&db, plan, standard, "s1.1.1"), Err(PlanError::Refused(_))));
        assert!(matches!(add_work_unit(&db, plan, standard, "nothing"), Err(PlanError::Refused(_))));

        let added = add_all_work_units(&db, plan, standard).unwrap();
        let units = read_work_units(&db, plan).unwrap();

        assert_eq!(units.len(), added + 1);
        assert_eq!(add_all_work_units(&db, plan, standard).unwrap(), 0);
    }

    #[test]
    fn a_work_unit_gets_its_parameters_and_becomes_a_functional_test() {
        let (_directory, db) = database();
        let (plan, standard) = plan_with_standard(&db);
        let first = add_work_unit(&db, plan, standard, "s1.1.1").unwrap();
        let second = add_work_unit(&db, plan, standard, "s1.1.2").unwrap();
        let document = add_document_place(&db, plan, "ST").unwrap();
        let parameters = WorkUnitParameters {
            dependencies: vec![first],
            input_product_documents: vec![document],
            input_functional: Vec::new(),
            ai_agent: None,
            guidance: "Read the ST.".to_owned(),
        };

        update_work_unit_parameters(&db, plan, second, &parameters).unwrap();

        let stored = read_work_unit(&db, second).unwrap();

        assert_eq!(stored.dependencies, vec![first]);
        assert_eq!(stored.input_functional, None);

        let proof = add_proof_place(&db, plan, second, "Scan of the ports").unwrap();

        assert_eq!(read_work_unit(&db, second).unwrap().input_functional, Some(vec![proof]));
    }

    #[test]
    fn a_cycle_blocks_the_plan_and_removing_a_work_unit_frees_it() {
        let (_directory, db) = database();
        let (plan, standard) = plan_with_standard(&db);
        let first = add_work_unit(&db, plan, standard, "s1.1.1").unwrap();
        let second = add_work_unit(&db, plan, standard, "s1.1.2").unwrap();
        let depends = |on: i64| WorkUnitParameters {
            dependencies: vec![on],
            input_product_documents: Vec::new(),
            input_functional: Vec::new(),
            ai_agent: None,
            guidance: String::new(),
        };

        update_work_unit_parameters(&db, plan, first, &depends(second)).unwrap();
        update_work_unit_parameters(&db, plan, second, &depends(first)).unwrap();

        assert!(read_evaluation_plan(&db, plan).unwrap().blocked);

        remove_work_unit(&db, plan, second).unwrap();

        assert!(!read_evaluation_plan(&db, plan).unwrap().blocked);
        assert!(!read_work_unit(&db, first).unwrap().cross_reference);
        assert!(read_work_unit(&db, first).unwrap().dependencies.is_empty());
    }
}
