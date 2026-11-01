//! Help for the tests of the evaluation: an evaluation of two work units,
//! the second depending on the first, both needing the ST, which is given;
//! an evaluation whose inputs are not given yet; a plan in planning to fill
//! with the AI.

use crate::db::models::ai::agent::create_agent;
use crate::db::models::evaluation::evaluation_plan::{
    create_evaluation, create_evaluation_plan, read_evaluation_plan, update_evaluation_plan,
};
use crate::db::models::evaluation::lab_proof::create_lab_proof;
use crate::db::models::evaluation::product_document::{read_product_document, update_product_document};
use crate::db::models::evaluation::work_unit::{create_work_unit, read_work_units, update_work_unit};
use crate::domain::models::evaluation::evaluation_plan::planning::{add_document_place, create_plan, update_plan};
use crate::domain::models::evaluation::evaluation_plan::work_units::add_all_work_units;
use crate::domain::models::evaluation::oscal_standard_document::import_standard;
use crate::infrastructure::storage::user_database::UserDatabase;
use crate::model::evaluation::evaluation_plan::EvaluationPlan;
use crate::model::evaluation::lab_proof::LabProof;
use crate::model::evaluation::product_document::ProductDocument;
use crate::model::evaluation::work_unit::WorkUnit;
use crate::objects::ai::integration::integration_store::create_integration;
use crate::test_support::database::{agent, integration, plan, unit};

/// The text of the ST of the evaluation.
pub const ST: &str = "This ST is the Security Target of the router R7.\n\nThe TOE is the router R7 with\nfirmware \
2.4.1. Its TSF filters packets.";

/// An evaluation of the plan of routers: ASE_INT.1-1 and ASE_INT.1-2, the
/// second depending on the first, both needing the ST, given with [`ST`];
/// an agent of the evaluation. Gives the evaluation and its two work units.
pub fn evaluation(db: &UserDatabase) -> (i64, i64, i64) {
    let routers = plan(db);
    let source = create_evaluation_plan(db, &routers).unwrap();
    let first = create_work_unit(db, source, &unit(&routers, "ASE_INT.1-1")).unwrap();

    create_work_unit(db, source, &WorkUnit { dependencies: vec![first], ..unit(&routers, "ASE_INT.1-2") }).unwrap();

    let general = create_agent(db, &agent(create_integration(db, &integration()).unwrap())).unwrap();
    let id = create_evaluation(db, source, "R7", "ACME", "K. K.", 1_793_000_000, Some(general)).unwrap();
    let document = read_evaluation_plan(db, id).unwrap().product_documents[0];
    let place = read_product_document(db, document).unwrap();
    let given = ProductDocument { content: Some(ST.as_bytes().to_vec()), version: Some("1.0".to_owned()), ..place };

    update_product_document(db, &given, Some(ST)).unwrap();

    let units = read_work_units(db, id).unwrap();

    (id, units[0].id, units[1].id)
}

/// An evaluation of the plan of routers with one work unit, ATE_IND.1-1,
/// that needs the ST and one proof "Scan", neither given yet. Gives the
/// evaluation.
pub fn evaluation_needing_inputs(db: &UserDatabase) -> i64 {
    let routers = plan(db);
    let source = create_evaluation_plan(db, &routers).unwrap();
    let scan = LabProof { id: 0, name: "Scan".to_owned(), proof: None, performed: None, tester: None, equipment: None };
    let needing = WorkUnit {
        input_product_documents: routers.product_documents.clone(),
        input_functional: Some(vec![create_lab_proof(db, &scan).unwrap()]),
        ..unit(&routers, "ATE_IND.1-1")
    };

    create_work_unit(db, source, &needing).unwrap();

    let general = create_agent(db, &agent(create_integration(db, &integration()).unwrap())).unwrap();

    create_evaluation(db, source, "R7", "ACME", "K. K.", 1_793_000_000, Some(general)).unwrap()
}

/// Writes the work unit `unit` anew, for a test that changes its results.
pub fn write(db: &UserDatabase, unit: &WorkUnit) {
    update_work_unit(db, unit).unwrap();
}

/// The sample catalog of NIST, four controls.
const NIST_SAMPLE: &str = include_str!("../../oscal/examples/basic-catalog.json");

/// A plan of the sample of NIST with its four work units, a document "ST"
/// and an agent of the planner; gives the plan and its work units.
pub fn plan_with_planner(db: &UserDatabase) -> (i64, Vec<i64>) {
    let standard = import_standard(db, NIST_SAMPLE).unwrap();
    let plan = create_plan(db, "Sample", "").unwrap();
    let stored = read_evaluation_plan(db, plan).unwrap();

    update_plan(db, &EvaluationPlan { oscal_documents: vec![standard], ..stored }).unwrap();
    add_document_place(db, plan, "ST").unwrap();
    add_all_work_units(db, plan, standard).unwrap();

    let integration = create_integration(db, &integration()).unwrap();
    let planner = create_agent(db, &agent(integration)).unwrap();
    let stored = read_evaluation_plan(db, plan).unwrap();

    update_evaluation_plan(db, &EvaluationPlan { ai_agent: Some(planner), ..stored.clone() }).unwrap();

    (plan, stored.work_units)
}
