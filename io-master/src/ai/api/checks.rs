//! The check of an AI connection: one short question, and a report of
//! what was sent, what came back and what the paths of the connection
//! found in it, for the evaluator on the tab AI of Settings.
//!
//! ```text
//! target()   the stored connection to check, with its key
//! create()   POST /connections/{id}/check   the report
//! record()   the check into the journal of the AI (task "check")
//! ```

use crate::db::error::DbError;
use crate::db::user::database::UserDb;

use super::completions::{self, Reply};
use super::provider::client;
use super::roles::{self, Model, ModelError};
use super::tasks::Task;

pub use super::provider::client::CheckReport;

/// Stored connection `id`, checked, with its key if one is stored. A
/// missing key is not refused here: the check reports it.
pub fn target(db: &UserDb, id: i64) -> Result<Model, ModelError> {
    roles::model_of(db, id, false)
}

/// Checks a connection with one short message.
pub async fn create(model: &Model) -> CheckReport {
    client::check(&model.connection, model.api_key.as_ref()).await
}

/// Writes a check into the journal of the AI: the body of the request
/// of the report (`report.request.body`) and its reply. Given as parts,
/// so that the report itself can go to the evaluator.
pub fn record(db: &UserDb, model: &Model, request: &str, reply: &Reply) -> Result<i64, DbError> {
    completions::write(db, model, Task::Check, request, reply)
}
