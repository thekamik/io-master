//! The error of a command, as the page gets it:
//!
//! ```text
//! {"kind": "wrong_credentials", "message": "the name or the password is wrong"}
//! ```
//!
//! `kind` lets the page react (for example show the login again when the
//! session has ended); `message` is the sentence it shows. No message holds
//! a password, a key or a recovery code.

use serde::ser::{Serialize, SerializeStruct, Serializer};

use crate::infrastructure::storage::error::DbError;
use crate::objects::ai_error::AiError;
use crate::objects::error::DomainError;
use crate::objects::user::user_data::user_data_rules::AccountError;
use crate::panels::logic::evaluation::error::EvaluationError;
use crate::panels::session::SessionBroken;

/// Why a command failed.
#[derive(Debug)]
pub enum ViewError {
    /// The account: a wrong name or password, a weak password, an
    /// installation that exists or does not.
    Account(AccountError),
    /// The session could not be used; the program should be started again.
    Session,
    /// Nobody is logged in: the page shows the login again.
    NotLoggedIn,
    /// The panel of the evaluation refused a change, or failed.
    Evaluation(EvaluationError),
    /// The way to the AI refused a question before it was sent, or failed
    /// to write it into the journal.
    Ai(AiError),
    /// A database refused or failed.
    Db(DbError),
    /// A rule of the data refused a change, or could not be checked
    /// (the rules of an object, `objects/error.rs`).
    Domain(DomainError),
    /// What the page sent, or a file the user chose, cannot be taken;
    /// holds the reason.
    Invalid(String),
}

impl From<EvaluationError> for ViewError {
    fn from(error: EvaluationError) -> ViewError {
        ViewError::Evaluation(error)
    }
}

impl From<AiError> for ViewError {
    fn from(error: AiError) -> ViewError {
        ViewError::Ai(error)
    }
}

impl From<DbError> for ViewError {
    fn from(error: DbError) -> ViewError {
        ViewError::Db(error)
    }
}

impl From<DomainError> for ViewError {
    fn from(error: DomainError) -> ViewError {
        ViewError::Domain(error)
    }
}

impl From<AccountError> for ViewError {
    fn from(error: AccountError) -> ViewError {
        ViewError::Account(error)
    }
}

impl From<SessionBroken> for ViewError {
    fn from(_: SessionBroken) -> ViewError {
        ViewError::Session
    }
}

impl ViewError {
    /// The kind of the error as the page knows it.
    ///
    /// Output: the kind (`"invalid"`, `"not_logged_in"`).
    ///
    /// Called by: `serialize` of `ViewError`, below.
    fn kind(&self) -> &'static str {
        match self {
            ViewError::Account(AccountError::NotInstalled) => "not_installed",
            ViewError::Account(AccountError::AlreadyInstalled) => "already_installed",
            ViewError::Account(
                AccountError::WrongCredentials | AccountError::WrongRecovery | AccountError::WrongPassword,
            ) => "wrong_credentials",
            ViewError::NotLoggedIn => "not_logged_in",
            ViewError::Account(
                AccountError::PasswordsDiffer
                | AccountError::WeakPassword(_)
                | AccountError::Code(_)
                | AccountError::Db(DbError::Inconsistent(_)),
            )
            | ViewError::Domain(
                DomainError::Refused(_)
                | DomainError::Oscal(_)
                | DomainError::Pdf(_)
                | DomainError::Db(DbError::Inconsistent(_)),
            )
            | ViewError::Db(DbError::Inconsistent(_))
            | ViewError::Ai(
                AiError::NoKey
                | AiError::Address(_)
                | AiError::NoQuestion
                | AiError::TooLong { .. }
                | AiError::TooManyMessages { .. }
                | AiError::Refused(_)
                | AiError::Domain(DomainError::Refused(_)),
            )
            | ViewError::Evaluation(EvaluationError::Db(DbError::Inconsistent(_)))
            | ViewError::Invalid(_) => "invalid",
            ViewError::Account(AccountError::Key(_) | AccountError::Db(_))
            | ViewError::Session
            | ViewError::Evaluation(EvaluationError::Db(_))
            | ViewError::Domain(DomainError::Db(_))
            | ViewError::Ai(_)
            | ViewError::Db(_) => "failed",
        }
    }

    /// The sentence the page shows.
    ///
    /// Output: the sentence ("the question is too long").
    ///
    /// Called by: `serialize` of `ViewError`, below.
    fn message(&self) -> String {
        match self {
            ViewError::Account(error) => error.to_string(),
            ViewError::Session => "the session of the window failed: start the program again".to_owned(),
            ViewError::NotLoggedIn => "nobody is logged in".to_owned(),
            ViewError::Evaluation(error) => error.to_string(),
            ViewError::Ai(error) => error.to_string(),
            ViewError::Db(error) => error.to_string(),
            ViewError::Domain(error) => error.to_string(),
            ViewError::Invalid(reason) => reason.clone(),
        }
    }
}

impl Serialize for ViewError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut error = serializer.serialize_struct("ViewError", 2)?;

        error.serialize_field("kind", self.kind())?;
        error.serialize_field("message", &self.message())?;
        error.end()
    }
}
