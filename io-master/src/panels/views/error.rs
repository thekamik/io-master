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

use crate::ai::error::AiError;
use crate::db::error::DbError;
use crate::panels::logic::agents::error::AgentError;
use crate::panels::logic::chat::error::ChatError;
use crate::panels::logic::evaluation::error::EvaluationError;
use crate::panels::logic::evaluation_plan::error::PlanError;
use crate::panels::logic::program_start::access::AccountError;
use crate::panels::logic::settings::error::SettingsError;
use crate::session::state::SessionBroken;

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
    /// The making of a plan refused or failed.
    Plan(PlanError),
    /// The panel of the agents refused a change, or failed.
    Agent(AgentError),
    /// The settings of the AI refused a change, or failed.
    Settings(SettingsError),
    /// The panel of the chat refused a chat or a question, or failed.
    Chat(ChatError),
    /// The panel of the evaluation refused a change, or failed.
    Evaluation(EvaluationError),
    /// The way to the AI refused a question before it was sent, or failed
    /// to write it into the journal.
    Ai(AiError),
    /// A database refused or failed.
    Db(DbError),
    /// What the page sent, or a file the user chose, cannot be taken;
    /// holds the reason.
    Invalid(String),
}

impl From<PlanError> for ViewError {
    fn from(error: PlanError) -> ViewError {
        ViewError::Plan(error)
    }
}

impl From<AgentError> for ViewError {
    fn from(error: AgentError) -> ViewError {
        ViewError::Agent(error)
    }
}

impl From<ChatError> for ViewError {
    fn from(error: ChatError) -> ViewError {
        ViewError::Chat(error)
    }
}

impl From<EvaluationError> for ViewError {
    fn from(error: EvaluationError) -> ViewError {
        ViewError::Evaluation(error)
    }
}

impl From<SettingsError> for ViewError {
    fn from(error: SettingsError) -> ViewError {
        ViewError::Settings(error)
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
    /// Called by: the serialization below.
    fn kind(&self) -> &'static str {
        match self {
            ViewError::Account(AccountError::NotInstalled) => "not_installed",
            ViewError::Account(AccountError::AlreadyInstalled) => "already_installed",
            ViewError::Account(AccountError::WrongCredentials) => "wrong_credentials",
            ViewError::Account(AccountError::PasswordsDiffer | AccountError::WeakPassword(_) | AccountError::Db(_)) => {
                "invalid"
            }
            ViewError::NotLoggedIn => "not_logged_in",
            ViewError::Plan(PlanError::Oscal(_) | PlanError::Refused(_) | PlanError::Db(DbError::Inconsistent(_)))
            | ViewError::Db(DbError::Inconsistent(_))
            | ViewError::Settings(
                SettingsError::Refused(_) | SettingsError::Ai(_) | SettingsError::Db(DbError::Inconsistent(_)),
            )
            | ViewError::Ai(AiError::NoKey | AiError::Address(_) | AiError::NoQuestion | AiError::TooLong { .. })
            | ViewError::Agent(
                AgentError::Refused(_) | AgentError::Pdf(_) | AgentError::Db(DbError::Inconsistent(_)),
            )
            | ViewError::Chat(
                ChatError::Refused(_)
                | ChatError::Ai(AiError::NoKey | AiError::Address(_) | AiError::NoQuestion | AiError::TooLong { .. })
                | ChatError::Db(DbError::Inconsistent(_)),
            )
            | ViewError::Evaluation(
                EvaluationError::Refused(_)
                | EvaluationError::Pdf(_)
                | EvaluationError::Ai(
                    AiError::NoKey | AiError::Address(_) | AiError::NoQuestion | AiError::TooLong { .. },
                )
                | EvaluationError::Db(DbError::Inconsistent(_)),
            )
            | ViewError::Invalid(_) => "invalid",
            ViewError::Account(AccountError::Key(_))
            | ViewError::Session
            | ViewError::Plan(PlanError::Db(_) | PlanError::Knowledge(_))
            | ViewError::Settings(SettingsError::Db(_))
            | ViewError::Agent(AgentError::Db(_))
            | ViewError::Chat(ChatError::Knowledge(_) | ChatError::Ai(_) | ChatError::Db(_))
            | ViewError::Evaluation(EvaluationError::Ai(_) | EvaluationError::Db(_))
            | ViewError::Ai(_)
            | ViewError::Db(_) => "failed",
        }
    }

    /// The sentence the page shows.
    ///
    /// Called by: the serialization below.
    fn message(&self) -> String {
        match self {
            ViewError::Account(error) => error.to_string(),
            ViewError::Session => "the session of the window failed: start the program again".to_owned(),
            ViewError::NotLoggedIn => "nobody is logged in".to_owned(),
            ViewError::Plan(error) => error.to_string(),
            ViewError::Settings(error) => error.to_string(),
            ViewError::Agent(error) => error.to_string(),
            ViewError::Chat(error) => error.to_string(),
            ViewError::Evaluation(error) => error.to_string(),
            ViewError::Ai(error) => error.to_string(),
            ViewError::Db(error) => error.to_string(),
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
