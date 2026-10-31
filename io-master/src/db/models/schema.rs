//! The tables of each database. Each `SCHEMA` is written in the file of its
//! model; this file only lists them for `storage/`, which makes the missing
//! tables whenever it opens a database, and so knows no model itself.
//!
//! **The tables are parted by the database they live in**, not by the
//! folder of their model: the system database is not encrypted, since it
//! is opened before the password is given, so it holds only what the login
//! needs; everything else is in the encrypted database of the user
//! (`docs/architecture.md`, section 5.3). Within a list the tables follow
//! the folders of `models/`; the order does not matter to SQLite, which
//! checks a foreign key when a row is written, not when its table is made.
//!
//! ```text
//! models/<area>/<model>.rs ──SCHEMA──► schema.rs ──USER_TABLES───► storage/user_database.rs
//!                                                 └─SYSTEM_TABLES─► storage/system_database.rs
//! ```

use crate::db::models::ai::{agent, api_key, chat, integration, journal, rag};
use crate::db::models::evaluation::{evaluation_plan, lab_proof, oscal_standard_document, product_document, work_unit};
use crate::db::models::user::{program_data, user_data, user_keys};

/// The tables of the database of the user (`users/<id>.v2.db`, encrypted).
pub const USER_TABLES: [&str; 12] = [
    // models/ai/
    agent::SCHEMA,
    api_key::SCHEMA,
    chat::SCHEMA,
    integration::SCHEMA,
    journal::SCHEMA,
    rag::SCHEMA,
    // models/evaluation/
    evaluation_plan::SCHEMA,
    lab_proof::SCHEMA,
    oscal_standard_document::SCHEMA,
    product_document::SCHEMA,
    work_unit::SCHEMA,
    // models/user/: the settings only (the installation is in the system database)
    program_data::SCHEMA,
];

/// The tables of the system database (`system.v2.db`, not encrypted).
pub const SYSTEM_TABLES: [&str; 3] = [
    // models/user/
    program_data::SYSTEM_SCHEMA,
    user_data::SCHEMA,
    user_keys::SCHEMA,
];
