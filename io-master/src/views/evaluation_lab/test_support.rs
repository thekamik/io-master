//! Help for the tests of the view Evaluation lab: "anna" logged in, her AI
//! a fake service, the basic catalog of NIST stored and the evaluation "e"
//! of the package of the tests (`test_support/db.rs`, `lab_package`), with a
//! note as evidence of its input "i"; and a channel that collects the events.

use std::sync::{Arc, Mutex};

use serde_json::Value;
use tauri::ipc::{Channel, InvokeResponseBody};

use crate::ai::api::connections;
use crate::ai::api::test_support::{FakeService, completions as connection_to};
use crate::app::test_support::{TestState, logged_in};
use crate::components::ai_work::view::AiEvent;
use crate::db_old::evaluation as db_evaluation;
use crate::model_old::api_key::ApiKey;
use crate::test_support::db::{basic_catalog, lab_evaluation, lab_package};

/// The text of the note given to input "i".
pub const NOTE: &str = "Anna owns the keys of the TOE.\nBob owns the audit log.";

/// The state with the evaluation "e"; its AI is `service`.
pub async fn ready(service: &FakeService) -> TestState {
    let test = logged_in().await;

    {
        let session = test.state.session().unwrap();
        let db = session.db();
        let id = connections::create(&db, &connection_to(service)).unwrap().id;
        connections::set_key(&db, id, &ApiKey::new("sk-test".to_owned()).unwrap()).unwrap();

        let catalog = basic_catalog();
        crate::db_old::oscal::create::document(&db, &catalog).unwrap();
        let package = lab_package(catalog.uuid());
        db_evaluation::create::evaluation(&db, &lab_evaluation(&package), &package).unwrap();
        super::evidence::add::add_note(&db, "e", "i", "Interview", NOTE).unwrap();
    }

    test
}

/// The text of the passage of the knowledge base of [`with_agent`].
pub const PASSAGE: &str = "Information security roles shall be defined and allocated by the management.";

/// An agent with knowledge (a base with one passage on roles,
/// `test_support/db.rs`) made the agent of the evaluation "e"; gives its id
/// (step 57.6).
pub fn with_agent(db: &crate::db_old::user::database::UserDb) -> i64 {
    let agent = crate::test_support::db::knowledge_agent(db, PASSAGE);
    db_evaluation::update::agent(db, "e", Some(agent)).unwrap();

    agent
}

/// A channel that collects the events it gets, as JSON.
pub fn collector() -> (Arc<Mutex<Vec<Value>>>, Channel<AiEvent>) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&events);

    let channel = Channel::new(move |body: InvokeResponseBody| {
        sink.lock().unwrap().push(body.deserialize::<Value>().unwrap());
        Ok(())
    });

    (events, channel)
}
