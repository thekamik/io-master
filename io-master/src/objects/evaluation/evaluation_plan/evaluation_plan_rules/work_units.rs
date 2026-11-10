//! The work units of a plan (`EvaluationPlan`) in planning, as its children:
//! added from a control of one of the standards of the plan (one, or all at
//! once), removed. After a work unit is removed the cross references of the
//! plan are marked again (`order.rs`: `check_order`).
//!
//! ```text
//! a standard of the plan ──catalog_controls (oscal_standard_document.rs)──► a control ──► a new work unit (create_work_unit)
//! ```
//!
//! A work unit made from a control has no documents, proofs or agent yet:
//! the planner gives them (`../work_unit_rules/parameters.rs`: `update_work_unit_parameters`).

use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::error::DomainError;
use crate::objects::evaluation::evaluation_plan::evaluation_plan_model::EvaluationPlan;
use crate::objects::evaluation::evaluation_plan::evaluation_plan_rules::order::check_order;
use crate::objects::evaluation::evaluation_plan::evaluation_plan_rules::planning::plan_in_planning;
use crate::objects::evaluation::evaluation_plan::evaluation_plan_store::read_evaluation_plan;
use crate::objects::evaluation::oscal_standard_document::oscal_standard_document_rules::{
    CatalogControl, catalog_controls,
};
use crate::objects::evaluation::oscal_standard_document::oscal_standard_document_store::read_oscal_standard_document;
use crate::objects::evaluation::work_unit::work_unit_model::WorkUnit;
use crate::objects::evaluation::work_unit::work_unit_store::{
    create_work_unit, create_work_units, delete_work_unit, read_work_units,
};

/// The controls of the standard `standard`, one of the standards of `plan`.
///
/// Input: `plan`, a stored plan; `standard`, the id of a stored standard.
/// Output: its controls in the order of the standard (`[CatalogControl {
/// label: "ADV_ARC.1-1", .. }, ...]`); `DomainError::Refused` for a standard
/// not of the plan or not JSON, `DomainError::Oscal` for one that is no
/// catalog.
///
/// Called by: [`available_controls`], [`add_work_unit`], [`add_all_work_units`].
/// Calls: `objects/evaluation/oscal_standard_document/oscal_standard_document_store.rs`:
/// `read_oscal_standard_document`; `objects/evaluation/oscal_standard_document/oscal_standard_document_rules.rs`:
/// `catalog_controls`.
fn controls_of(db: &UserDatabase, plan: &EvaluationPlan, standard: i64) -> Result<Vec<CatalogControl>, DomainError> {
    ensure!(
        plan.oscal_documents.contains(&standard),
        DomainError::Refused("the standard is not one of the plan".to_owned())
    );

    let document = read_oscal_standard_document(db, standard)?;
    let json = serde_json::from_str(&document.doc)
        .map_err(|error| DomainError::Refused(format!("the stored standard is no JSON: {error}")))?;

    Ok(catalog_controls(&json)?)
}

/// A new work unit of the standard `standard` made from its control
/// `control`: no dependencies, documents, proofs or agent yet.
///
/// Input: `standard`, the id of a standard of the plan; `control`, one of
/// its controls.
/// Output: the work unit, its id 0, to be created.
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
        proof_quotes: Vec::new(),
        verdict: None,
        author: None,
    }
}

/// Whether the plan holds the work unit of the control `oscal_key` of the
/// standard `standard` already.
///
/// Input: `units`, the work units of the plan; `standard`, the id of a
/// standard; `oscal_key`, the key of a control ("adv_arc.1-1").
/// Output: `true` when one of `units` is that control.
///
/// Called by: [`available_controls`], [`add_work_unit`], [`add_all_work_units`].
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
/// Input: `plan`, the id of a stored plan; `standard`, the id of one of its
/// standards.
/// Output: the controls (`[CatalogControl { label: "ADV_ARC.1-2", .. }]`);
/// the errors of [`controls_of`], an error of the database.
///
/// Called by: `panels/views/evaluation_plan/work_units.rs`: `work_unit_controls`.
/// Calls: `objects/evaluation/evaluation_plan/evaluation_plan_store.rs`: `read_evaluation_plan`; [`controls_of`];
/// `objects/evaluation/work_unit/work_unit_store.rs`: `read_work_units`; [`holds`].
pub fn available_controls(db: &UserDatabase, plan: i64, standard: i64) -> Result<Vec<CatalogControl>, DomainError> {
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
/// standard `standard`, as its last.
///
/// Input: `plan`, the id of a plan in planning; `standard`, the id of one of
/// its standards; `oscal_key`, the key of a control of it.
/// Output: the id of the new work unit (`14`); `DomainError::Refused` when
/// the plan holds it already, the standard has no such control, or the plan
/// is not in planning; the errors of [`controls_of`].
///
/// Called by: `panels/views/evaluation_plan/work_units.rs`: `work_unit_add`.
/// Calls: `objects/evaluation/evaluation_plan/evaluation_plan_rules/planning.rs`: `plan_in_planning`;
/// [`controls_of`], [`holds`], [`unit_of`]; `objects/evaluation/work_unit/work_unit_store.rs`: `read_work_units`,
/// `create_work_unit`.
pub fn add_work_unit(db: &UserDatabase, plan: i64, standard: i64, oscal_key: &str) -> Result<i64, DomainError> {
    let stored = plan_in_planning(db, plan)?;
    let controls = controls_of(db, &stored, standard)?;

    ensure!(
        !holds(&read_work_units(db, plan)?, standard, oscal_key),
        DomainError::Refused(format!("the plan holds {oscal_key} already"))
    );

    for control in &controls {
        if control.oscal_key == oscal_key {
            return Ok(create_work_unit(db, plan, &unit_of(standard, control))?);
        }
    }

    Err(DomainError::Refused(format!("the standard has no control {oscal_key}")))
}

/// Adds to the plan `plan` every control of its standard `standard` it does
/// not hold yet, in the order of the standard ("add all").
///
/// Input: `plan`, the id of a plan in planning; `standard`, the id of one of
/// its standards.
/// Output: how many were added (`37`); the refusals of a plan not in
/// planning, the errors of [`controls_of`].
///
/// Called by: `panels/views/evaluation_plan/work_units.rs`: `work_unit_add_all`.
/// Calls: `objects/evaluation/evaluation_plan/evaluation_plan_rules/planning.rs`: `plan_in_planning`;
/// [`controls_of`], [`holds`], [`unit_of`]; `objects/evaluation/work_unit/work_unit_store.rs`: `read_work_units`,
/// `create_work_units` (all of them in one transaction).
pub fn add_all_work_units(db: &UserDatabase, plan: i64, standard: i64) -> Result<usize, DomainError> {
    let stored = plan_in_planning(db, plan)?;
    let controls = controls_of(db, &stored, standard)?;
    let units = read_work_units(db, plan)?;
    let mut new = Vec::with_capacity(controls.len());

    for control in &controls {
        if !holds(&units, standard, &control.oscal_key) {
            new.push(unit_of(standard, control));
        }
    }

    Ok(create_work_units(db, plan, &new)?)
}

/// Removes the work unit `unit` from the plan `plan` in planning, with the
/// dependencies of the others on it, and marks the cross references again.
///
/// Input: `plan`, the id of a plan in planning; `unit`, the id of one of its
/// work units.
/// Output: nothing; `DomainError::Refused` for a work unit not of the plan
/// or a plan not in planning, an error of the database.
///
/// Called by: `panels/views/evaluation_plan/work_units.rs`: `work_unit_remove`.
/// Calls: `objects/evaluation/evaluation_plan/evaluation_plan_rules/planning.rs`: `plan_in_planning`;
/// `objects/evaluation/work_unit/work_unit_store.rs`: `delete_work_unit`;
/// `objects/evaluation/evaluation_plan/evaluation_plan_rules/order.rs`: `check_order`.
pub fn remove_work_unit(db: &UserDatabase, plan: i64, unit: i64) -> Result<(), DomainError> {
    let stored = plan_in_planning(db, plan)?;

    ensure!(stored.work_units.contains(&unit), DomainError::Refused("the work unit is not one of the plan".to_owned()));

    delete_work_unit(db, unit)?;
    check_order(db, plan)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Tests of the work units of a plan: added from the sample of NIST (one,
    //! all), a cycle of their dependencies, removed.

    use super::*;

    use crate::objects::evaluation::evaluation_plan::evaluation_plan_rules::planning::{create_plan, update_plan};
    use crate::objects::evaluation::oscal_standard_document::oscal_standard_document_rules::import_standard;
    use crate::objects::evaluation::work_unit::work_unit_rules::parameters::{
        WorkUnitParameters, update_work_unit_parameters,
    };
    use crate::objects::evaluation::work_unit::work_unit_store::read_work_unit;
    use crate::test_support::database::database;

    const NIST_SAMPLE: &str = include_str!("../../../../../oscal/examples/basic-catalog.json");

    /// Whether `controls` hold the control `oscal_key`.
    fn holds_key(controls: &[CatalogControl], oscal_key: &str) -> bool {
        controls.iter().any(|control| control.oscal_key == oscal_key)
    }

    /// A plan with the sample of NIST as its standard.
    fn plan_with_standard(db: &UserDatabase) -> (i64, i64) {
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
        assert!(matches!(add_work_unit(&db, plan, standard, "s1.1.1"), Err(DomainError::Refused(_))));
        assert!(matches!(add_work_unit(&db, plan, standard, "nothing"), Err(DomainError::Refused(_))));

        let added = add_all_work_units(&db, plan, standard).unwrap();
        let units = read_work_units(&db, plan).unwrap();

        assert_eq!(units.len(), added + 1);
        assert_eq!(add_all_work_units(&db, plan, standard).unwrap(), 0);
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
