//! Help for the tests of `db/`: an AI connection that keeps the rules,
//! the API key of the tests, an entry of the journal of the AI, and a
//! quote and a proposal of the evaluation support, a catalog of NIST, and
//! the system database with an installation and the key of an account, and
//! a work package as the Evaluation lab imports it.

use serde_json::{Value, json};

use crate::db::ai::journal::rows::Entry;
use crate::db::encryption::keys::WrappedKey;
use crate::db::encryption::recovery::RecoveryWrappedKey;
use crate::db::evaluation_support::create::{self, NewProposal};
use crate::db::system::database::SystemDb;
use crate::db::user::database::UserDb;
use crate::model_old::ai::Connection;
use crate::model_old::api_key::ApiKey;
use crate::model_old::evaluation::{Evaluation, EvaluationFields, LabTask, Package, PackageTask, TaskState};
use crate::model_old::evaluation_support::{ProposalKind, QuoteDraft};
use crate::model_old::installation::{ID_LENGTH, Installation};
use crate::model_old::oscal::OscalDocument;
use crate::model_old::task_scheme::{self, Block, BlockFields, BlockKind, OutputCondition, OutputFormat};
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

/// The installation of the tests.
pub fn installation() -> Installation {
    Installation { id: [5; ID_LENGTH] }
}

/// A copy of the key of an account wrapped with a recovery code, as the
/// system database keeps it.
pub fn recovery_key() -> RecoveryWrappedKey {
    RecoveryWrappedKey { salt: vec![5; 16], nonce: vec![6; 12], ciphertext: vec![7; 32], tag: vec![8; 16] }
}

/// A wrapped key of an account, as the system database keeps it.
pub fn wrapped_key() -> WrappedKey {
    WrappedKey {
        salt: vec![1; 16],
        memory_kib: 256,
        iterations: 1,
        nonce: vec![2; 12],
        ciphertext: vec![3; 32],
        tag: vec![4; 16],
    }
}

/// A new system database of the installation of the tests in `directory`.
pub fn system_db(directory: &TempDir) -> SystemDb {
    SystemDb::create(directory.path(), &installation()).unwrap()
}

/// A block of kind `kind` named `name` for the package of the tests.
fn lab_block(uuid: &str, kind: BlockKind, name: &str, condition: OutputCondition) -> Block {
    let fields = BlockFields {
        name: name.to_owned(),
        evidence: if kind == BlockKind::Input { "The list of roles".to_owned() } else { String::new() },
        instructions: "Check it.".to_owned(),
        target: String::new(),
        format: if kind == BlockKind::Output { OutputFormat::Latex } else { OutputFormat::Text },
        condition,
    };

    Block { uuid: uuid.to_owned(), kind, x: 0, y: 0, fields, proposal: None }
}

/// A work package of one task of the basic catalog of NIST (`catalog`,
/// requirement s1.1.1): the requirement "r", the input "i", the outputs
/// "o" (pETR, always) and "n" (Observation Report, only when not met).
pub fn lab_package(catalog: &str) -> Package {
    let blocks = vec![
        lab_block("r", BlockKind::Requirement, "", OutputCondition::Always),
        lab_block("i", BlockKind::Input, "Roles document", OutputCondition::Always),
        lab_block("o", BlockKind::Output, "pETR", OutputCondition::Always),
        lab_block("n", BlockKind::Output, "Observation Report", OutputCondition::NotMet),
    ];
    let connections = vec![
        task_scheme::Connection { from: "i".to_owned(), to: "r".to_owned() },
        task_scheme::Connection { from: "r".to_owned(), to: "o".to_owned() },
        task_scheme::Connection { from: "r".to_owned(), to: "n".to_owned() },
    ];
    let task = LabTask {
        uuid: "t".to_owned(),
        catalog_uuid: catalog.to_owned(),
        control_id: "s1.1.1".to_owned(),
        label: "1.1.1".to_owned(),
        title: "Information security roles and responsibilities".to_owned(),
        state: TaskState::Todo,
    };

    Package {
        package_uuid: "w".to_owned(),
        project_title: "CC".to_owned(),
        scope: "A smart card reader.".to_owned(),
        name: "Roles".to_owned(),
        evaluator: "Anna".to_owned(),
        note: String::new(),
        tasks: vec![PackageTask { task, blocks, connections, related: Vec::new() }],
    }
}

/// The evaluation "e" of `package` for the product "Reader".
pub fn lab_evaluation(package: &Package) -> Evaluation {
    Evaluation {
        uuid: "e".to_owned(),
        package_uuid: package.package_uuid.clone(),
        project_title: package.project_title.clone(),
        scope: package.scope.clone(),
        package_name: package.name.clone(),
        evaluator: package.evaluator.clone(),
        note: package.note.clone(),
        fields: EvaluationFields { product: "Reader".to_owned(), product_version: "2.0".to_owned() },
    }
}

/// An agent with knowledge named "ISO agent" whose one base holds one
/// passage, `text` (page 7 of "ISO 27002"); gives its id (step 57.6).
pub fn knowledge_agent(db: &UserDb, text: &str) -> i64 {
    let base = crate::db::knowledge::create::base(db, "ISO", "").unwrap();
    let passage = crate::model_old::knowledge::Passage { page: Some(7), label: String::new(), text: text.to_owned() };
    let kind = crate::model_old::knowledge::SourceKind::Pdf;
    crate::db::knowledge::create::source(db, base, kind, "ISO 27002", &[passage]).unwrap();
    let fields = crate::model_old::knowledge::AgentFields {
        name: "ISO agent".to_owned(),
        instructions: "You know ISO 27002.".to_owned(),
        glossary: String::new(),
        passages: 4,
        characters: 4000,
        bases: vec![base],
    };

    crate::db::knowledge::create::agent(db, &fields).unwrap()
}
