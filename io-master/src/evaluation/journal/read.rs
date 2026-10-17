//! The journal of an evaluation (P3 of `architecture.md`: everything
//! is traceable): every import, every text of the AI, every
//! acceptance, correction, rejection and every result written.
//!
//! The journal is **only added to**. An entry is written by the
//! storage itself, in the same transaction as the change it records
//! (`db/evaluations/create.rs`, `journal`); nothing changes or removes
//! an entry, and the database refuses it as well (two triggers,
//! `db/evaluations/schema.rs`): an entry is removed only with its
//! whole evaluation (`projects.rs`, `delete`).

use crate::db::error::DbError;
use crate::db::evaluations::read;
use crate::db::user::database::UserDb;

use crate::model::evaluations::JournalEntry;

/// The journal of an evaluation, the oldest entry first.
pub fn of(db: &UserDb, evaluation: i64) -> Result<Vec<JournalEntry>, DbError> {
    read::journal(db, evaluation)
}
