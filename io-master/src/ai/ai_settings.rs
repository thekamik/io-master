//! The AI settings of an evaluator: the API key of Anthropic and
//! the chosen model, both kept in the encrypted database of the
//! evaluator (`db/user_db.rs`):
//!
//! ```text
//! secrets:   anthropic.api_key   the API key
//! settings:  ai.model            the API name of the model
//! ```
//!
//! The database is opened only with the key that the password of
//! the evaluator unwraps (`db/keys.rs`), so the API key is protected
//! at least by that password, and by every further factor once
//! they exist (steps 8 and 9 of `docs/todo.md`).
//!
//! Storing is done only after a check with Anthropic
//! ([`Anthropic::check`]); this file only reads and writes.
//!
//! [`Anthropic::check`]: super::anthropic::Anthropic::check

use crate::db::db::DbError;
use crate::db::user_db::UserDb;

use super::api_key::ApiKey;
use super::model::Model;

/// Name of the secret with the API key.
const API_KEY: &str = "anthropic.api_key";

/// Name of the setting with the model.
const MODEL: &str = "ai.model";

/// The stored API key, or `None` when there is none.
///
/// A stored text that is no longer a valid key (impossible unless
/// the database was changed by hand) counts as no key.
pub fn api_key(db: &UserDb) -> Result<Option<ApiKey>, DbError> {
    Ok(db
        .secret(API_KEY)?
        .and_then(|text| ApiKey::new(text.as_str().to_owned()).ok()))
}

/// Whether an API key is stored, without reading it out.
pub fn has_api_key(db: &UserDb) -> Result<bool, DbError> {
    Ok(db.secret(API_KEY)?.is_some())
}

/// Stores the API key, replacing an earlier one.
pub fn store_api_key(db: &UserDb, key: &ApiKey) -> Result<(), DbError> {
    db.set_secret(API_KEY, key.expose())
}

/// Removes the API key.
pub fn remove_api_key(db: &UserDb) -> Result<(), DbError> {
    db.remove_secret(API_KEY)
}

/// The chosen model; the default when none was chosen, or when the
/// stored one is no longer offered.
pub fn model(db: &UserDb) -> Result<Model, DbError> {
    Ok(db
        .setting(MODEL)?
        .and_then(|id| Model::from_id(&id))
        .unwrap_or(Model::DEFAULT))
}

/// Stores the chosen model.
pub fn store_model(db: &UserDb, model: Model) -> Result<(), DbError> {
    db.set_setting(MODEL, model.id())
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "ai_settings_tests.rs"]
mod tests;
