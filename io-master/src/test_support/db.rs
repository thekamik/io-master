//! Help for the tests of `db/`: an AI connection that keeps the rules,
//! the API key of the tests, an entry of the journal of the AI, and a
//! quote and a proposal of the evaluation support, a catalog of NIST, and
//! the system database with an installation and the key of an account.

use serde_json::{Value, json};

use crate::db::ai::journal::rows::Entry;
use crate::db::encryption::keys::WrappedKey;
use crate::db::evaluation_support::create::{self, NewProposal};
use crate::db::system::database::SystemDb;
use crate::db::user::database::UserDb;
use crate::model::ai::Connection;
use crate::model::api_key::ApiKey;
use crate::model::evaluation_support::{ProposalKind, QuoteDraft};
use crate::model::installation::{ID_LENGTH, Installation, Role};
use crate::model::oscal::OscalDocument;
use crate::test_support::TempDir;

/// The API key of the tests; it must never be found in the file of the
/// database.
pub const SECRET_KEY: &str = "sk-very-secret-key-of-anna";

/// A connection named `name` that keeps the rules.
pub fn named(name: &str) -> Connection {
    Connection { name: name.to_owned(), model: "llama".to_owned(), ..Connection::template() }
}

/// The API key of the tests.
pub fn key() -> ApiKey {
    ApiKey::new(SECRET_KEY.to_owned()).unwrap()
}

/// An entry of the journal of the AI of the task `task`, as a question of
/// the chat would leave it (its id and time are given by the database).
pub fn journal_entry(task: &str) -> Entry {
    Entry {
        id: 0,
        time: 0,
        task: task.to_owned(),
        role: "consultation".to_owned(),
        connection: "Local".to_owned(),
        model: "gemma4".to_owned(),
        request: "{\"messages\": []}".to_owned(),
        answer: "Hello".to_owned(),
        ending: "complete".to_owned(),
        note: None,
        input_tokens: 12,
        output_tokens: 3,
    }
}

/// A quote of page 2 of the document `source`, found on its page.
pub fn quote_draft(source: i64, text: &str) -> QuoteDraft {
    QuoteDraft { source_document_id: source, page: 2, text: text.to_owned(), found: true }
}

/// The content of the proposal of the tests: a control of the group ADV.
pub fn proposal_content() -> Value {
    json!({"id": "ADV_FSP.4", "title": "Complete functional specification"})
}

/// Stores the proposal of the tests for the document "doc", with one quote
/// of `source`; returns its id.
pub fn stored_proposal(db: &UserDb, source: i64) -> i64 {
    let quotes = [QuoteDraft { found: false, ..quote_draft(source, "ADV_FSP.4") }];

    create::proposal(
        db,
        &NewProposal {
            document_uuid: "doc",
            kind: ProposalKind::Control,
            parent_id: Some("ADV"),
            content: &proposal_content(),
            quotes: &quotes,
            note: "The component of ADV.",
            model: "gemma4",
        },
    )
    .unwrap()
}

/// The basic catalog of the examples of NIST (`oscal/examples/`).
pub fn basic_catalog() -> OscalDocument {
    OscalDocument::from_json(include_str!("../../oscal/examples/basic-catalog.json")).unwrap()
}

/// The installation of the tests: a workstation.
pub fn installation() -> Installation {
    Installation { id: [5; ID_LENGTH], role: Role::Workstation }
}

/// A wrapped key of an account, as the system database keeps it.
pub fn wrapped_key() -> WrappedKey {
    WrappedKey {
        salt: vec![1; 16],
        memory_kib: 256,
        iterations: 1,
        with_device_secret: false,
        nonce: vec![2; 12],
        ciphertext: vec![3; 32],
        tag: vec![4; 16],
    }
}

/// A new system database of the installation of the tests in `directory`.
pub fn system_db(directory: &TempDir) -> SystemDb {
    SystemDb::create(directory.path(), &installation()).unwrap()
}
