//! The model a question goes to (user, 2026-10-17: the AI acts in
//! roles, and every question is asked in a task of one role,
//! `tasks.rs`; the role is written into the journal of the AI). Every
//! role uses the active connection of Settings: a connection of its
//! own per role was designed in step 38 and never offered on the page,
//! and was removed on 2026-10-24 (step 53).
//!
//! ```text
//! role                 used by (tasks.rs)
//! text_analysis        the catalogs, the schemes, the proposals of tasks and related elements, the fragments
//! summary              the texts of the outputs
//! consultation         the chat, the chat of an input
//! formal_analysis      the assessment of a requirement
//! model()              the active connection, checked, with its key
//! ```

use std::fmt;

use crate::db_old::ai::connection as db_connection;
use crate::db_old::error::DbError;
use crate::db_old::user::database::UserDb;

use super::connection::Checked;
use crate::model_old::ai::ConnectionError;
use crate::model_old::api_key::ApiKey;

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
    /// No connection is active.
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

/// The active connection, checked, with its key.
///
/// Called by: every work of the AI (the chat, the Converter, the Evaluation plan, the Evaluation lab).
/// Calls: `db_connection::read::active`, [`model_of`].
pub fn model(db: &UserDb) -> Result<Model, ModelError> {
    let id = db_connection::read::active(db)?.ok_or(ModelError::NoConnection)?;

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

    use crate::db_old::ai::connection::{create, update};
    use crate::model_old::ai::Connection;
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
    fn a_question_goes_to_the_active_connection_with_its_key() {
        let test = database();
        assert!(matches!(model(&test.db), Err(ModelError::NoConnection)));

        let first = create::connection(&test.db, &connection("First")).unwrap().id;
        create::connection(&test.db, &connection("Second")).unwrap();

        // The first connection is the active one; it needs its key.
        assert!(matches!(model(&test.db), Err(ModelError::NoKey(name)) if name == "First"));
        update::api_key(&test.db, first, &ApiKey::new("sk-key".to_owned()).unwrap()).unwrap();
        assert_eq!(model(&test.db).unwrap().name, "First");
    }

    #[test]
    fn a_check_runs_without_a_key() {
        let test = database();
        let id = create::connection(&test.db, &connection("Local")).unwrap().id;

        assert!(matches!(model_of(&test.db, id, true), Err(ModelError::NoKey(_))));
        assert!(model_of(&test.db, id, false).unwrap().api_key.is_none());
    }
}
