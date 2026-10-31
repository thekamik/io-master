//! The integrations with AI services (`Integration`), in the table
//! `ai_integrations` of the database of the user: create, read, update;
//! never deleted (user, 2026-10-28). Their API keys are kept apart
//! (`api_key.rs`).
//!
//! ```text
//! ai_integrations   id, name (unique), url, model, protocol, ca_certificate, context_size
//! ```

use rusqlite::{Row, named_params};

use crate::db::error::DbError;
use crate::db::storage::user_database::UserDatabase;
use crate::model::ai::integration::{Integration, Protocol};

/// The most integrations of the user.
pub const MAX_INTEGRATIONS: usize = 50;

/// The table, made when it is missing (`schema.rs`, `USER_TABLES`).
pub const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS ai_integrations (
        id             INTEGER PRIMARY KEY,
        name           TEXT NOT NULL UNIQUE,
        url            TEXT NOT NULL,
        model          TEXT NOT NULL,
        protocol       TEXT NOT NULL CHECK (protocol IN ('openai_chat', 'anthropic_messages')),
        ca_certificate TEXT,
        context_size   INTEGER NOT NULL
    );
";

/// The columns every read selects.
const COLUMNS: &str = "id, name, url, model, protocol, ca_certificate, context_size";

/// The name of a protocol in the column `protocol` (`Protocol::OpenAiChat` →
/// "openai_chat").
///
/// Called by: [`create_integration`], [`update_integration`].
fn protocol_name(protocol: Protocol) -> &'static str {
    match protocol {
        Protocol::OpenAiChat => "openai_chat",
        Protocol::AnthropicMessages => "anthropic_messages",
    }
}

/// The protocol of its name in the column `protocol` ("openai_chat" →
/// `Protocol::OpenAiChat`); a name the program never writes is
/// `DbError::Corrupt`.
///
/// Called by: [`integration_of`].
fn protocol_of(name: &str) -> Result<Protocol, DbError> {
    match name {
        "openai_chat" => Ok(Protocol::OpenAiChat),
        "anthropic_messages" => Ok(Protocol::AnthropicMessages),
        _ => Err(DbError::Corrupt { table: "ai_integrations", column: "protocol", value: name.to_owned() }),
    }
}

/// Refuses an integration without a name or an address, or with no room
/// for a question (a context of 0 tokens).
///
/// Input: `integration`, the integration to be stored.
///
/// Output: nothing when it is good; otherwise `DbError::Inconsistent`.
///
/// Called by: [`create_integration`], [`update_integration`].
fn check_integration(integration: &Integration) -> Result<(), DbError> {
    ensure!(!integration.name.is_empty(), DbError::Inconsistent("an integration without a name".to_owned()));
    ensure!(!integration.url.is_empty(), DbError::Inconsistent("an integration without an address".to_owned()));
    ensure!(integration.context_size > 0, DbError::Inconsistent("an integration without a context".to_owned()));

    Ok(())
}

/// Stores a new integration with an AI service and gives its id.
///
/// Input:
/// - `db`: the open database of the user.
/// - `integration`: the new integration, its `id` 0 (the database gives
///   it): a name, the address ("https://api.openai.com/v1/chat/completions"),
///   the model, the protocol, an optional CA certificate, the size of the
///   context.
///
/// Output: the id of the integration (3). An error when it is refused
/// ([`check_integration`]), when the user has [`MAX_INTEGRATIONS`] already,
/// or when its name is taken (`UNIQUE`).
///
/// Called by: `panels/logic/settings/integrations.rs`: `add_integration`.
/// Calls: [`check_integration`], [`protocol_name`].
pub fn create_integration(db: &UserDatabase, integration: &Integration) -> Result<i64, DbError> {
    ensure!(integration.id == 0, DbError::Inconsistent("a new integration has an id already".to_owned()));
    check_integration(integration)?;

    let connection = db.connection();
    let count: i64 = connection.query_row("SELECT COUNT(*) FROM ai_integrations", [], |row| row.get(0))?;

    ensure!(
        usize::try_from(count).map_err(|error| DbError::Inconsistent(error.to_string()))? < MAX_INTEGRATIONS,
        DbError::Inconsistent(format!("more than {MAX_INTEGRATIONS} integrations"))
    );

    connection.execute(
        "INSERT INTO ai_integrations (name, url, model, protocol, ca_certificate, context_size)
         VALUES (:name, :url, :model, :protocol, :ca_certificate, :context_size)",
        named_params! {
            ":name": integration.name,
            ":url": integration.url,
            ":model": integration.model,
            ":protocol": protocol_name(integration.protocol),
            ":ca_certificate": integration.ca_certificate,
            ":context_size": integration.context_size,
        },
    )?;

    Ok(connection.last_insert_rowid())
}

/// The model of one row of `ai_integrations`.
///
/// Called by: [`read_integration`], [`read_integrations`].
/// Calls: [`protocol_of`].
fn integration_of(row: &Row<'_>) -> Result<Integration, DbError> {
    let protocol: String = row.get("protocol")?;

    Ok(Integration {
        id: row.get("id")?,
        name: row.get("name")?,
        url: row.get("url")?,
        model: row.get("model")?,
        protocol: protocol_of(&protocol)?,
        ca_certificate: row.get("ca_certificate")?,
        context_size: row.get("context_size")?,
    })
}

/// Reads the integration `id`: where a question is sent and how.
///
/// Input: `db`, the open database of the user; `id`, the id of the
/// integration (3).
///
/// Output: the integration; `DbError::NoRow` when there is none.
///
/// Called by: `ai/ask.rs`: `prepare`; `api_key.rs`: `origin_of`;
/// `panels/logic/agents/agent.rs`: `checked`, `new_agent`;
/// `panels/logic/evaluation_plan/fill.rs`: `fill_preview`;
/// `panels/logic/settings/integrations.rs`: `activate_integration`,
/// `change_integration`, `set_key`; `panels/views/agents/agent.rs`:
/// `agent_list`.
/// Calls: [`integration_of`].
pub fn read_integration(db: &UserDatabase, id: i64) -> Result<Integration, DbError> {
    let connection = db.connection();
    let mut statement = connection.prepare(&format!("SELECT {COLUMNS} FROM ai_integrations WHERE id = :id"))?;
    let mut rows = statement.query(named_params! { ":id": id })?;
    let Some(row) = rows.next()? else {
        return Err(DbError::NoRow { table: "ai_integrations", id });
    };

    integration_of(row)
}

/// Reads every integration of the user, by their names.
///
/// Input: `db`, the open database of the user.
///
/// Output: the integrations, ordered by name; an error when the table holds
/// more than [`MAX_INTEGRATIONS`] (a database the program did not write).
///
/// Called by: `panels/views/agents/agent.rs`: `agent_get`;
/// `panels/views/settings/integrations.rs`: `integration_list`.
/// Calls: [`integration_of`].
pub fn read_integrations(db: &UserDatabase) -> Result<Vec<Integration>, DbError> {
    let connection = db.connection();
    let mut statement = connection.prepare(&format!("SELECT {COLUMNS} FROM ai_integrations ORDER BY name"))?;
    let mut rows = statement.query([])?;
    let mut integrations = Vec::new();

    for _ in 0..=MAX_INTEGRATIONS {
        let Some(row) = rows.next()? else {
            return Ok(integrations);
        };

        integrations.push(integration_of(row)?);
    }

    Err(DbError::Inconsistent(format!("more than {MAX_INTEGRATIONS} integrations in the database")))
}

/// Writes the integration `integration.id` anew. A new address does not
/// carry its key along: the key stays bound to the address it was given for
/// (`api_key.rs`), so it must be given again.
///
/// Input:
/// - `db`: the open database of the user.
/// - `integration`: the integration as it is to be kept, its `id` that of a
///   stored one.
///
/// Output: nothing; an error when it is refused ([`check_integration`]) or
/// its name is taken, or `DbError::NoRow` when there is no integration
/// `integration.id`.
///
/// Called by: `panels/logic/settings/integrations.rs`: `change_integration`.
/// Calls: [`check_integration`], [`protocol_name`].
pub fn update_integration(db: &UserDatabase, integration: &Integration) -> Result<(), DbError> {
    check_integration(integration)?;

    let changed = db.connection().execute(
        "UPDATE ai_integrations SET name = :name, url = :url, model = :model, protocol = :protocol,
                ca_certificate = :ca_certificate, context_size = :context_size
         WHERE id = :id",
        named_params! {
            ":id": integration.id,
            ":name": integration.name,
            ":url": integration.url,
            ":model": integration.model,
            ":protocol": protocol_name(integration.protocol),
            ":ca_certificate": integration.ca_certificate,
            ":context_size": integration.context_size,
        },
    )?;

    ensure!(changed == 1, DbError::NoRow { table: "ai_integrations", id: integration.id });

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Tests of the integrations: stored, read, changed, refused.

    use super::*;

    use crate::test_support::database::{database, integration};

    #[test]
    fn an_integration_is_stored_read_and_changed() {
        let (_directory, db) = database();
        let id = create_integration(&db, &integration()).unwrap();
        let mut read = read_integration(&db, id).unwrap();

        assert_eq!(read, Integration { id, ..integration() });

        read.protocol = Protocol::AnthropicMessages;
        read.ca_certificate = Some("-----BEGIN CERTIFICATE-----".to_owned());
        update_integration(&db, &read).unwrap();

        assert_eq!(read_integrations(&db).unwrap(), vec![read]);
    }

    #[test]
    fn what_is_no_integration_is_refused() {
        let (_directory, db) = database();

        assert!(matches!(
            create_integration(&db, &Integration { name: String::new(), ..integration() }),
            Err(DbError::Inconsistent(_))
        ));
        assert!(matches!(
            create_integration(&db, &Integration { context_size: 0, ..integration() }),
            Err(DbError::Inconsistent(_))
        ));
        assert!(matches!(read_integration(&db, 4), Err(DbError::NoRow { .. })));
        assert!(matches!(update_integration(&db, &Integration { id: 4, ..integration() }), Err(DbError::NoRow { .. })));
    }
}
