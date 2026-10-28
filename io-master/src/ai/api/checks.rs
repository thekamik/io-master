//! The check of an AI connection: one short question, and a report of
//! what was sent, what came back and what the paths of the connection
//! found in it, for the evaluator on the tab AI of Settings.
//!
//! ```text
//! target()   the stored connection to check, with its key
//! create()   POST /connections/{id}/check   the report, and the check
//!            in the journal of the AI (task "check")
//! ```

use std::sync::{Arc, Mutex, PoisonError};

use crate::db::error::DbError;
use crate::db::user::database::UserDb;

use super::completions;
use super::provider::client;
use super::roles::{self, Model, ModelError};
use crate::model_old::ai::Task;

pub use super::provider::client::CheckReport;

/// Stored connection `id`, checked, with its key if one is stored. A
/// missing key is not refused here: the check reports it.
pub fn target(db: &UserDb, id: i64) -> Result<Model, ModelError> {
    roles::model_of(db, id, false)
}

/// Checks a connection with one short message, and writes the check
/// into the journal of the AI in `db` (the body of the request of the
/// report, never a header, and its reply). Returns the report and the
/// number of the entry, or why it could not be written.
pub async fn create(db: &Arc<Mutex<UserDb>>, model: &Arc<Model>) -> (CheckReport, Result<i64, DbError>) {
    let report = client::check(&model.connection, model.api_key.as_ref()).await;

    let (db, model) = (db.clone(), model.clone());
    let (request, reply) = (report.request.body.to_string(), report.reply.clone());
    let journal = tokio::task::spawn_blocking(move || {
        let db = db.lock().unwrap_or_else(PoisonError::into_inner);

        completions::write(&db, &model, Task::Check, &request, &reply)
    })
    .await
    .unwrap_or_else(|_| Err(DbError::Inconsistent("the entry of the journal was not written".to_owned())));

    (report, journal)
}
