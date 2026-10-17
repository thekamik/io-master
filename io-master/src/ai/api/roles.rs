//! The roles of the AI (user, 2026-10-17): the AI acts in several
//! roles, and every question to it is asked in one of them.
//!
//! ```text
//! role                 used by (tasks.rs)                 answer   quotes   memory
//! text_analysis        chapters, table of contents,       JSON     checked  never shortened
//!                      later phase 1 of the evaluation
//! summary              later: AI notes, drafts of outputs text     -        by parameters
//! consultation         the chat                           text     -        by parameters
//! formal_analysis      later: phase 2 of the evaluation   fields   checked  never shortened
//! ```
//!
//! A role is fixed by the program: the form of its answer, whether its
//! quotes are checked, and what the memory may do with its text (the
//! quotes of a standard are never shortened, user 2026-10-17). What the
//! evaluator chooses is kept in the database: the connection a role
//! uses (`db/ai/roles/`; none: the active connection) and the
//! instructions of each task (`tasks.rs`).
//!
//! ```text
//! list()             GET  /roles
//! connection()       GET  /roles/{role}/connection
//! set_connection()   PUT  /roles/{role}/connection
//! model()            the connection a role uses now, checked, with its key
//! ```

use std::fmt;

use crate::db::ai::connection::read as connections;
use crate::db::ai::roles::{read as roles_read, update as roles_update};
use crate::db::error::DbError;
use crate::db::user::database::UserDb;

use super::api_key::ApiKey;
use super::connection::{Checked, ConnectionError};

/// A role of the AI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    /// Analysis of a text and pointing to its fragments.
    TextAnalysis,
    /// Summarising parts of a text.
    Summary,
    /// Consultation: the chat.
    Consultation,
    /// Formal analysis: the assessment of a requirement.
    FormalAnalysis,
}

/// The form of the answer a role expects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnswerForm {
    /// One JSON object, read by the task (`evaluation/analysis.rs`).
    Json,
    /// Free text.
    Text,
    /// Text for named fields (the four results of a requirement).
    Fields,
}

/// What the memory may do with the text a role works on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRule {
    /// Word for word, never shortened: quotes of a standard,
    /// fragments of a document.
    Verbatim,
    /// May be shortened or summarised, only as its parameters say.
    ByParameters,
}

impl Role {
    /// Every role.
    pub const ALL: [Role; 4] = [Role::TextAnalysis, Role::Summary, Role::Consultation, Role::FormalAnalysis];

    /// The name kept in the database.
    pub fn name(self) -> &'static str {
        match self {
            Role::TextAnalysis => "text_analysis",
            Role::Summary => "summary",
            Role::Consultation => "consultation",
            Role::FormalAnalysis => "formal_analysis",
        }
    }

    pub fn from_name(name: &str) -> Option<Role> {
        Role::ALL.into_iter().find(|role| role.name() == name)
    }

    /// The title shown to the evaluator.
    pub fn title(self) -> &'static str {
        match self {
            Role::TextAnalysis => "Analysis of a text and its fragments",
            Role::Summary => "Summary",
            Role::Consultation => "Consultation",
            Role::FormalAnalysis => "Formal analysis",
        }
    }

    pub fn answer(self) -> AnswerForm {
        match self {
            Role::TextAnalysis => AnswerForm::Json,
            Role::Summary | Role::Consultation => AnswerForm::Text,
            Role::FormalAnalysis => AnswerForm::Fields,
        }
    }

    /// Whether the quotes of its answers are checked against their
    /// pages.
    pub fn quotes_checked(self) -> bool {
        matches!(self, Role::TextAnalysis | Role::FormalAnalysis)
    }

    pub fn memory(self) -> MemoryRule {
        match self {
            Role::TextAnalysis | Role::FormalAnalysis => MemoryRule::Verbatim,
            Role::Summary | Role::Consultation => MemoryRule::ByParameters,
        }
    }
}

/// The connection a question goes over: checked, with its key.
#[derive(Debug)]
pub struct Model {
    /// The name of the connection (for the journal).
    pub name: String,
    pub connection: Checked,
    /// `None` when none is stored (the connection needs none).
    pub api_key: Option<ApiKey>,
}

/// Why there is no model to ask.
#[derive(Debug)]
pub enum ModelError {
    /// No connection is given to the role, and none is active.
    NoConnection,
    /// The connection breaks a rule (a database changed by hand, or a
    /// rule of a newer version); holds its name and why.
    BrokenConnection(String, ConnectionError),
    /// The connection uses an API key and none is stored; holds its
    /// name.
    NoKey(String),
    /// The database failed.
    Db(DbError),
}

impl fmt::Display for ModelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelError::NoConnection => {
                formatter.write_str("no AI connection is chosen; set one up in Settings, tab AI")
            }
            ModelError::BrokenConnection(name, error) => {
                write!(formatter, "the connection {name} cannot be used: {error}; correct it in Settings, tab AI")
            }
            ModelError::NoKey(name) => {
                write!(
                    formatter,
                    "the connection {name} uses an API key and none is stored; enter one in Settings, tab AI"
                )
            }
            ModelError::Db(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for ModelError {}

impl From<DbError> for ModelError {
    fn from(error: DbError) -> ModelError {
        ModelError::Db(error)
    }
}

/// Every role.
pub fn list() -> Vec<Role> {
    Role::ALL.to_vec()
}

/// The connection given to `role`; `None`: it uses the active one.
pub fn connection(db: &UserDb, role: Role) -> Result<Option<i64>, DbError> {
    roles_read::connection(db, role)
}

/// Gives `role` the connection `id`, or none (the active one); returns
/// `false` when the connection does not exist.
pub fn set_connection(db: &UserDb, role: Role, id: Option<i64>) -> Result<bool, DbError> {
    roles_update::connection(db, role, id)
}

/// The connection `role` uses now (its own, else the active one),
/// checked, with its key.
pub fn model(db: &UserDb, role: Role) -> Result<Model, ModelError> {
    let id = match roles_read::connection(db, role)? {
        Some(id) => id,
        None => connections::active(db)?.ok_or(ModelError::NoConnection)?,
    };

    model_of(db, id, true)
}

/// Connection `id`, checked, with its key. When `needs_key`, a
/// connection that uses a key and has none is refused; a check of a
/// connection (`checks.rs`) runs without, and reports it.
pub(super) fn model_of(db: &UserDb, id: i64, needs_key: bool) -> Result<Model, ModelError> {
    let stored = connections::get(db, id)?.ok_or(ModelError::NoConnection)?;
    let name = stored.connection.name.clone();
    let connection = stored
        .connection
        .check()
        .map_err(|error| ModelError::BrokenConnection(name.clone(), error))?;
    let api_key = connections::api_key(db, id)?;

    if needs_key && connection.needs_key() && api_key.is_none() {
        return Err(ModelError::NoKey(name));
    }

    Ok(Model {
        name,
        connection,
        api_key,
    })
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "roles_tests.rs"]
mod tests;
