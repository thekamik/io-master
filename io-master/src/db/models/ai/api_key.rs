//! The API keys of the integrations (`ApiKey`), in the table
//! `ai_api_keys` of the encrypted database of the user, apart from the
//! integrations: create, read, update; never deleted (user, 2026-10-28).
//!
//! **A key is bound to the address it was given for**: its origin (the
//! scheme, the host and the port of the address of its integration) is
//! kept with it, and a key is never read for an integration whose address
//! has changed since, so that a key never travels to another service. A
//! new key given for the new address replaces it.
//!
//! ```text
//! ai_api_keys   integration (one key at most), origin, key
//! ```

use reqwest::Url;
use rusqlite::named_params;
use zeroize::Zeroizing;

use crate::db::error::DbError;
use crate::db::models::ai::integration::read_integration;
use crate::db::storage::user_database::UserDatabase;
use crate::model::ai::api_key::ApiKey;

/// The longest key accepted, in characters: keys are 30 to 120 characters,
/// the limit stops a pasted document.
pub const MAX_KEY: usize = 512;

/// The table, made when it is missing (`schema.rs`, `USER_TABLES`).
pub const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS ai_api_keys (
        integration INTEGER PRIMARY KEY REFERENCES ai_integrations(id),
        origin      TEXT NOT NULL,
        key         TEXT NOT NULL
    );
";

/// The origin of the address of the integration `integration`: the scheme,
/// the host and the port its key is bound to.
///
/// Called by: [`write_api_key`], [`read_api_key`].
/// Calls: `integration::read_integration`.
fn origin_of(db: &UserDatabase, integration: i64) -> Result<String, DbError> {
    let url = read_integration(db, integration)?.url;
    let parsed = Url::parse(&url).map_err(|error| DbError::Inconsistent(format!("the address {url}: {error}")))?;
    let origin = parsed.origin().ascii_serialization();

    ensure!(origin != "null", DbError::Inconsistent(format!("the address {url} has no origin")));

    Ok(origin)
}

/// Writes the key of an integration with the origin of its address:
/// made, or replaced.
///
/// Called by: [`create_api_key`], [`update_api_key`].
/// Calls: [`origin_of`].
fn write_api_key(db: &UserDatabase, key: &ApiKey) -> Result<(), DbError> {
    let text: &str = key.key.as_str();

    ensure!(!text.is_empty(), DbError::Inconsistent("an empty API key".to_owned()));
    ensure!(text.chars().count() <= MAX_KEY, DbError::Inconsistent(format!("an API key over {MAX_KEY} characters")));

    let origin = origin_of(db, key.integration)?;

    db.connection().execute(
        "INSERT INTO ai_api_keys (integration, origin, key) VALUES (:integration, :origin, :key)
         ON CONFLICT (integration) DO UPDATE SET origin = :origin, key = :key",
        named_params! { ":integration": key.integration, ":origin": origin, ":key": text },
    )?;

    Ok(())
}

/// Stores the key of an integration that has none, bound to its present
/// address.
///
/// Called by: nothing yet (the views rewritten on `db/`).
/// Calls: [`write_api_key`].
pub fn create_api_key(db: &UserDatabase, key: &ApiKey) -> Result<(), DbError> {
    let count: i64 = db.connection().query_row(
        "SELECT COUNT(*) FROM ai_api_keys WHERE integration = :integration",
        named_params! { ":integration": key.integration },
        |row| row.get(0),
    )?;

    ensure!(count == 0, DbError::Inconsistent("the integration has a key already".to_owned()));

    write_api_key(db, key)
}

/// Reads the key of the integration `integration`; `None` when it has
/// none, or when its address has changed since the key was given (the key
/// is not sent to another service).
///
/// Called by: nothing yet (the AI layer rewritten on `db/`).
/// Calls: [`origin_of`].
pub fn read_api_key(db: &UserDatabase, integration: i64) -> Result<Option<ApiKey>, DbError> {
    let origin = origin_of(db, integration)?;
    let connection = db.connection();
    let mut statement = connection.prepare("SELECT origin, key FROM ai_api_keys WHERE integration = :integration")?;
    let mut rows = statement.query(named_params! { ":integration": integration })?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    let bound: String = row.get("origin")?;

    if bound != origin {
        return Ok(None);
    }

    Ok(Some(ApiKey { integration, key: Zeroizing::new(row.get("key")?) }))
}

/// Replaces the key of an integration, bound to its present address.
///
/// Called by: nothing yet (the views rewritten on `db/`).
/// Calls: [`write_api_key`].
pub fn update_api_key(db: &UserDatabase, key: &ApiKey) -> Result<(), DbError> {
    write_api_key(db, key)
}

#[cfg(test)]
mod tests {
    //! Tests of the API keys: stored, read, bound to their address.

    use super::*;

    use crate::db::models::ai::integration::{create_integration, update_integration};
    use crate::test_support::database::{database, integration};

    fn key(integration: i64, text: &str) -> ApiKey {
        ApiKey { integration, key: Zeroizing::new(text.to_owned()) }
    }

    #[test]
    fn a_key_is_stored_read_and_replaced() {
        let (_directory, db) = database();
        let id = create_integration(&db, &integration()).unwrap();

        assert!(read_api_key(&db, id).unwrap().is_none());

        create_api_key(&db, &key(id, "sk-one")).unwrap();

        assert_eq!(read_api_key(&db, id).unwrap().unwrap().key.as_str(), "sk-one");
        assert!(matches!(create_api_key(&db, &key(id, "sk-two")), Err(DbError::Inconsistent(_))));

        update_api_key(&db, &key(id, "sk-two")).unwrap();

        assert_eq!(read_api_key(&db, id).unwrap().unwrap().key.as_str(), "sk-two");
    }

    #[test]
    fn a_key_is_not_read_for_another_address() {
        let (_directory, db) = database();
        let id = create_integration(&db, &integration()).unwrap();

        create_api_key(&db, &key(id, "sk-one")).unwrap();

        let moved = crate::model::ai::integration::Integration {
            id,
            url: "https://other.example/v1/chat/completions".to_owned(),
            ..integration()
        };

        update_integration(&db, &moved).unwrap();

        assert!(read_api_key(&db, id).unwrap().is_none());
        assert!(matches!(create_api_key(&db, &key(id, "")), Err(DbError::Inconsistent(_))));
    }
}
