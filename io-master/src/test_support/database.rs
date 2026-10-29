//! Help for the tests of `db/`: a new database of the user in a temporary
//! folder, removed with it; a plan, a work unit, an integration and an
//! agent to store in it.

use crate::db::database::{self, Database};
use crate::db::encryption::database_key::DatabaseKey;
use crate::db::evaluation::oscal_standard_document::create_oscal_standard_document;
use crate::db::evaluation::product_document::create_product_document;
use crate::model::ai::agent::Agent;
use crate::model::ai::integration::{Integration, Protocol};
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
        planning_agent: None,
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

/// An integration with a local model, not stored yet.
pub fn integration() -> Integration {
    Integration {
        id: 0,
        name: "Ollama".to_owned(),
        url: "http://127.0.0.1:11434/v1/chat/completions".to_owned(),
        model: "llama3.1".to_owned(),
        protocol: Protocol::OpenAiChat,
        ca_certificate: None,
        context_size: 32_768,
    }
}

/// An agent of the integration `integration`, without knowledge, not
/// stored yet.
pub fn agent(integration: i64) -> Agent {
    Agent {
        id: 0,
        name: "CC evaluator".to_owned(),
        integration,
        instructions: "You assess work units of the CEM.".to_owned(),
        knowledge_bases: Vec::new(),
        glossary: Vec::new(),
        max_passages: 8,
        max_characters: 12_000,
    }
}

/// A work unit `id` depending on `dependencies`, never stored: for the
/// tests of the order of the work alone.
pub fn node(id: i64, dependencies: &[i64]) -> WorkUnit {
    WorkUnit {
        id,
        reference: 1,
        label: format!("WU {id}"),
        oscal_key: format!("wu-{id}"),
        requirement: String::new(),
        guidance: String::new(),
        dependencies: dependencies.to_vec(),
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
