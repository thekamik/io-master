//! Help for the tests of the evaluation: an evaluation of two work units,
//! the second depending on the first, both needing the ST, which is given.

use crate::db::models::ai::agent::create_agent;
use crate::db::models::ai::integration::create_integration;
use crate::db::models::evaluation::evaluation_plan::{create_evaluation, create_evaluation_plan, read_evaluation_plan};
use crate::db::models::evaluation::product_document::{read_product_document, update_product_document};
use crate::db::models::evaluation::work_unit::{create_work_unit, read_work_units, update_work_unit};
use crate::db::storage::user_database::UserDatabase;
use crate::model::evaluation::product_document::ProductDocument;
use crate::model::evaluation::work_unit::WorkUnit;
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

/// Writes the work unit `unit` anew, for a test that changes its results.
pub fn write(db: &UserDatabase, unit: &WorkUnit) {
    update_work_unit(db, unit).unwrap();
}
