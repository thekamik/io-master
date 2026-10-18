//! What the long works of the business share (step 40): the stop
//! signal, the error, and the steps on the session. A long work (the
//! loading of documents, an analysis with the AI, a reply of the chat)
//! runs step by step; between its steps it waits for files or for the
//! model and holds no lock of the session (`session.rs`). It reports
//! its progress to a function of the caller, which returns `false` when
//! nobody listens any more: the work then stops as if it was stopped.
//!
//! ```text
//! Cancel       the stop signal ("Stop" in the window, or the logout)
//! WorkError    why a long work could not go on
//! on_db()      one step on the database, on a thread for blocking work
//! ```

use std::fmt;
use std::sync::Arc;

use tokio::sync::watch;

use crate::ai::api::roles::ModelError;
use crate::business::chats::conversation::SendError;
use crate::business::session::Session;
use crate::business::standards::error::DocumentError;
use crate::db::error::DbError;
use crate::db::user::database::UserDb;

/// The stop signal of a long work: `true` once it is to stop.
#[derive(Debug, Clone)]
pub struct Cancel(watch::Receiver<bool>);

impl Cancel {
    pub fn new(signal: watch::Receiver<bool>) -> Cancel {
        Cancel(signal)
    }

    /// Whether the work is to stop.
    pub fn is_cancelled(&self) -> bool {
        *self.0.borrow()
    }

    /// The signal itself, for `ai/api/`, which stops a reply with it.
    pub fn signal(&self) -> &watch::Receiver<bool> {
        &self.0
    }
}

/// Why a long work could not go on.
#[derive(Debug)]
pub enum WorkError {
    /// What was asked is not possible; the reason, for the evaluator.
    Invalid(String),
    /// A document of a standard.
    Document(DocumentError),
    /// No AI connection can be used for the task.
    Model(ModelError),
    /// A message of the chat was refused.
    Message(SendError),
    /// The database failed.
    Db(DbError),
    /// A step was interrupted (its thread ended without a result).
    Interrupted,
}

impl fmt::Display for WorkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorkError::Invalid(reason) => formatter.write_str(reason),
            WorkError::Document(error) => write!(formatter, "{error}"),
            WorkError::Model(error) => write!(formatter, "{error}"),
            WorkError::Message(error) => write!(formatter, "{error}"),
            WorkError::Db(error) => write!(formatter, "{error}"),
            WorkError::Interrupted => formatter.write_str("the work was interrupted"),
        }
    }
}

impl std::error::Error for WorkError {}

impl From<DocumentError> for WorkError {
    fn from(error: DocumentError) -> WorkError {
        WorkError::Document(error)
    }
}

impl From<ModelError> for WorkError {
    fn from(error: ModelError) -> WorkError {
        WorkError::Model(error)
    }
}

impl From<SendError> for WorkError {
    fn from(error: SendError) -> WorkError {
        WorkError::Message(error)
    }
}

impl From<DbError> for WorkError {
    fn from(error: DbError) -> WorkError {
        WorkError::Db(error)
    }
}

/// Runs one step on the database of `session`, on a thread for
/// blocking work.
pub async fn on_db<T: Send + 'static>(
    session: &Arc<Session>,
    step: impl FnOnce(&UserDb) -> Result<T, WorkError> + Send + 'static,
) -> Result<T, WorkError> {
    let session = session.clone();

    tokio::task::spawn_blocking(move || step(&session.db())).await.unwrap_or(Err(WorkError::Interrupted))
}
