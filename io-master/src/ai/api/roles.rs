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

use crate::db::ai::connection as db_connection;
use crate::db::ai::roles as db_roles;
use crate::db::error::DbError;
use crate::db::user::database::UserDb;

use super::connection::Checked;
use crate::model::ai::{ConnectionError, Role};
use crate::model::api_key::ApiKey;

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
    db_roles::read::connection(db, role)
}

/// Gives `role` the connection `id`, or none (the active one); returns
/// `false` when the connection does not exist.
pub fn set_connection(db: &UserDb, role: Role, id: Option<i64>) -> Result<bool, DbError> {
    db_roles::update::connection(db, role, id)
}

/// The connection `role` uses now (its own, else the active one),
/// checked, with its key.
pub fn model(db: &UserDb, role: Role) -> Result<Model, ModelError> {
    let id = match db_roles::read::connection(db, role)? {
        Some(id) => id,
        None => db_connection::read::active(db)?.ok_or(ModelError::NoConnection)?,
    };

    model_of(db, id, true)
}

/// Connection `id`, checked, with its key. When `needs_key`, a
/// connection that uses a key and has none is refused; a check of a
/// connection (`checks.rs`) runs without, and reports it.
pub(super) fn model_of(db: &UserDb, id: i64, needs_key: bool) -> Result<Model, ModelError> {
    let stored = db_connection::read::get(db, id)?.ok_or(ModelError::NoConnection)?;
    let name = stored.connection.name.clone();
    let connection = stored.connection.check().map_err(|error| ModelError::BrokenConnection(name.clone(), error))?;
    let api_key = db_connection::read::api_key(db, id)?;

    if needs_key && connection.needs_key() && api_key.is_none() {
        return Err(ModelError::NoKey(name));
    }

    Ok(Model { name, connection, api_key })
}

#[cfg(test)]
mod tests {
    //! Unit tests of `roles.rs`.

    use super::*;

    use crate::model::ai::MemoryRule;

    use crate::db::ai::connection::{create, delete, update};
    use crate::model::ai::Connection;
    use crate::test_support::standards::database;

    /// A connection named `name` that needs a key.
    fn connection(name: &str) -> Connection {
        Connection {
            name: name.to_owned(),
            url: "http://127.0.0.1:11434/v1/chat/completions".to_owned(),
            model: name.to_lowercase(),
            headers: "authorization: Bearer {{api_key}}".to_owned(),
            ..Connection::template()
        }
    }

    #[test]
    fn the_roles_are_those_the_user_named() {
        let titles: Vec<&str> = list().iter().map(|role| role.title()).collect();

        assert_eq!(titles, ["Analysis of a text and its fragments", "Summary", "Consultation", "Formal analysis"]);

        for role in Role::ALL {
            assert_eq!(Role::from_name(role.name()), Some(role));
        }

        // The quotes of a standard are never shortened.
        assert_eq!(Role::TextAnalysis.memory(), MemoryRule::Verbatim);
        assert_eq!(Role::FormalAnalysis.memory(), MemoryRule::Verbatim);
        assert!(Role::TextAnalysis.quotes_checked() && !Role::Consultation.quotes_checked());
    }

    #[test]
    fn a_role_uses_its_own_connection_else_the_active_one() {
        let test = database();
        assert!(matches!(model(&test.db, Role::Consultation), Err(ModelError::NoConnection)));

        let first = create::connection(&test.db, &connection("First")).unwrap().id;
        let second = create::connection(&test.db, &connection("Second")).unwrap().id;
        let key = ApiKey::new("sk-key".to_owned()).unwrap();
        update::api_key(&test.db, first, &key).unwrap();

        // The first connection is the active one.
        assert_eq!(model(&test.db, Role::TextAnalysis).unwrap().name, "First");

        // A role of its own: needs the key of its connection.
        assert!(set_connection(&test.db, Role::FormalAnalysis, Some(second)).unwrap());
        assert_eq!(connection_of(&test, Role::FormalAnalysis), Some(second));
        assert!(matches!(model(&test.db, Role::FormalAnalysis), Err(ModelError::NoKey(name)) if name == "Second"));

        // Its connection deleted: the role uses the active one again.
        delete::connection(&test.db, second).unwrap();
        assert_eq!(connection_of(&test, Role::FormalAnalysis), None);
        assert_eq!(model(&test.db, Role::FormalAnalysis).unwrap().name, "First");

        // A connection that does not exist is refused.
        assert!(!set_connection(&test.db, Role::Summary, Some(9_999)).unwrap());
    }

    fn connection_of(test: &crate::test_support::standards::TestDb, role: Role) -> Option<i64> {
        super::connection(&test.db, role).unwrap()
    }

    #[test]
    fn a_check_runs_without_a_key() {
        let test = database();
        let id = create::connection(&test.db, &connection("Local")).unwrap().id;

        assert!(matches!(model_of(&test.db, id, true), Err(ModelError::NoKey(_))));
        assert!(model_of(&test.db, id, false).unwrap().api_key.is_none());
    }
}
