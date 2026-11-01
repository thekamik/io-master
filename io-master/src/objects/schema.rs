//! The tables of each database. Each `SCHEMA` is written in the store of
//! its model (`<object>_store.rs`; until they move into `objects/`, the files
//! of `db/models/`); this file only lists them. The caller that opens a
//! database gives its list to `infrastructure/storage/`, which makes the
//! missing tables, and so knows no model itself.
//!
//! **The tables are parted by the database they live in**, not by the
//! folder of their model: the system database is not encrypted, since it
//! is opened before the password is given, so it holds only what the login
//! needs; everything else is in the encrypted database of the user
//! (`docs/architecture.md`, section 5.3). Within a list the tables follow
//! the folders of their models; the order does not matter to SQLite, which
//! checks a foreign key when a row is written, not when its table is made.
//!
//! ```text
//! <object>_store.rs ──SCHEMA──► schema.rs ──USER_TABLES, SYSTEM_TABLES──► user_data_rules.rs
//!   (the first start, the login) ──create / open(data_dir, ..., tables)──► infrastructure/storage/
//! ```

use crate::db::models::ai::{agent, chat, rag};
use crate::db::models::evaluation::{evaluation_plan, lab_proof, oscal_standard_document, product_document, work_unit};
use crate::objects::ai::api_key::api_key_store;
use crate::objects::ai::integration::integration_store;
use crate::objects::ai::journal::journal_store;
use crate::objects::user::program_data::program_data_store;
use crate::objects::user::user_data::user_data_store;
use crate::objects::user::user_keys::user_keys_store;

/// The tables of the database of the user (`users/<id>.v2.db`, encrypted).
pub const USER_TABLES: [&str; 12] = [
    // db/models/ai/
    agent::SCHEMA,
    chat::SCHEMA,
    rag::SCHEMA,
    // objects/ai/
    api_key_store::SCHEMA,
    integration_store::SCHEMA,
    journal_store::SCHEMA,
    // db/models/evaluation/
    evaluation_plan::SCHEMA,
    lab_proof::SCHEMA,
    oscal_standard_document::SCHEMA,
    product_document::SCHEMA,
    work_unit::SCHEMA,
    // objects/user/: the settings only (the installation is in the system database)
    program_data_store::SCHEMA,
];

/// The tables of the system database (`system.v2.db`, not encrypted).
pub const SYSTEM_TABLES: [&str; 3] = [
    // objects/user/
    program_data_store::SYSTEM_SCHEMA,
    user_data_store::SCHEMA,
    user_keys_store::SCHEMA,
];
