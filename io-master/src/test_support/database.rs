//! Help for the tests of `db/`: a new database of the user in a temporary
//! folder, removed with it; a plan and a work unit to store in it.

use crate::db::database::{self, Database};
use crate::db::encryption::database_key::DatabaseKey;
use crate::db::evaluation::oscal_standard_document::create_oscal_standard_document;
use crate::db::evaluation::product_document::create_product_document;
use crate::model::evaluation::evaluation_plan::{EvaluationPlan, PlanState};
use crate::model::evaluation::oscal_standard_document::OscalStandardDocument;
use crate::model::evaluation::product_document::ProductDocument;
use crate::model::evaluation::work_unit::{Verdict, WorkUnit};
use crate::security::random::fill_random;
use crate::test_support::TempDir;

/// A new, empty database; the folder is kept as long as it is.
pub fn database() -> (TempDir, Database) {
    let directory = TempDir::new();
    let key = DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap();
    let database = database::create(directory.path(), 1, &key).unwrap();

    (directory, database)
}

/// A plan of routers in planning, with one standard and one document.
pub fn plan(db: &Database) -> EvaluationPlan {
    let standard =
        OscalStandardDocument { id: 0, title: "CEM".to_owned(), doc: "{}".to_owned(), version: "1".to_owned() };
    let document = ProductDocument { id: 0, name: "ST".to_owned(), content: None, version: None };

    EvaluationPlan {
        id: 0,
        name: "Routers".to_owned(),
        description: String::new(),
        state: PlanState::Plan,
        source_plan: None,
        product: String::new(),
        client: String::new(),
        evaluator: String::new(),
        started: None,
        oscal_documents: vec![create_oscal_standard_document(db, &standard).unwrap()],
        product_documents: vec![create_product_document(db, &document).unwrap()],
        possible_verdicts: vec![Verdict::Pass, Verdict::Fail],
        work_units: Vec::new(),
        blocked: false,
    }
}

/// A work unit of the standard of `plan`, without dependencies.
pub fn unit(plan: &EvaluationPlan, label: &str) -> WorkUnit {
    WorkUnit {
        id: 0,
        reference: plan.oscal_documents[0],
        label: label.to_owned(),
        oscal_key: label.to_owned(),
        requirement: "The evaluator shall examine the ST.".to_owned(),
        guidance: String::new(),
        dependencies: Vec::new(),
        cross_reference: false,
        input_product_documents: plan.product_documents.clone(),
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
