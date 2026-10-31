//! The tables of each database, in the order they are made: a table comes
//! after the tables it names. Each `SCHEMA` is written in the file of its
//! model; this file only lists them for `storage/`, which makes the missing
//! tables whenever it opens a database, and so knows no model itself.
//!
//! ```text
//! models/<area>/<model>.rs ──SCHEMA──► schema.rs ──USER_TABLES───► storage/user_database.rs
//!                                                 └─SYSTEM_TABLES─► storage/system_database.rs
//! ```

use crate::db::models::ai::{agent, api_key, chat, integration, journal, rag};
use crate::db::models::evaluation::{evaluation_plan, lab_proof, oscal_standard_document, product_document, work_unit};
use crate::db::models::user::{program_data, user_data, user_keys};

/// The tables of the database of the user (`users/<id>.v2.db`).
pub const USER_TABLES: [&str; 12] = [
    integration::SCHEMA,
    api_key::SCHEMA,
    program_data::SCHEMA,
    rag::SCHEMA,
    agent::SCHEMA,
    oscal_standard_document::SCHEMA,
    product_document::SCHEMA,
    lab_proof::SCHEMA,
    evaluation_plan::SCHEMA,
    work_unit::SCHEMA,
    chat::SCHEMA,
    journal::SCHEMA,
];

/// The tables of the system database (`system.v2.db`).
pub const SYSTEM_TABLES: [&str; 3] = [program_data::SYSTEM_SCHEMA, user_data::SCHEMA, user_keys::SCHEMA];
