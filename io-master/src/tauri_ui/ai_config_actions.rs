//! The tab AI of Settings: the AI connections of the evaluator, their
//! API keys, the active connection, and the check of a connection.
//!
//! **A key is never sent back to the page**: the page learns only
//! whether one is stored. Saving a connection needs no answer of the
//! service (a local model may be switched off while it is set up);
//! "Check connection" ([`check_connection`]) asks the service and
//! reports what happened. "AI help" ([`field_instruction`]) gives
//! the instruction for a chatbot that helps to fill one field.

use crate::ai::api_key::ApiKey;
use crate::ai::client;
use crate::ai::connection::Field;
use crate::ai::connections;
use crate::ai::field_help;

use super::error::UiError;
use super::serializer::ai_config_view::{AiConfigView, CheckView, ConnectionForm, SavedView};
use super::state::UiState;
use super::user_data::with_user_db;

/// The connection does not exist.
fn no_such_connection() -> UiError {
    UiError::Invalid("the connection does not exist".to_owned())
}

/// The tab AI of Settings.
pub async fn ai_config(state: &UiState) -> Result<AiConfigView, UiError> {
    with_user_db(state, |db| Ok(AiConfigView::new(connections::list(db)?, connections::active(db)?))).await
}

/// Saves a connection (a new one when `id` is `None`).
pub async fn save_connection(state: &UiState, id: Option<i64>, form: ConnectionForm) -> Result<SavedView, UiError> {
    let connection = form.into_connection()?;

    with_user_db(state, move |db| {
        let saved = connections::save(db, id, &connection)?;

        let notice = if saved.key_removed {
            "The connection was saved. Its API key was removed, because the address changed; enter the key again \
             if the new address needs it."
        } else {
            "The connection was saved."
        };

        Ok(SavedView {
            config: AiConfigView::new(connections::list(db)?, connections::active(db)?),
            id: saved.id,
            notice: notice.to_owned(),
        })
    })
    .await
}

/// Deletes a connection with its API key.
pub async fn delete_connection(state: &UiState, id: i64) -> Result<AiConfigView, UiError> {
    if !with_user_db(state, move |db| Ok(connections::delete(db, id)?)).await? {
        return Err(no_such_connection());
    }

    ai_config(state).await
}

/// Makes a connection the one the chat uses.
pub async fn activate_connection(state: &UiState, id: i64) -> Result<AiConfigView, UiError> {
    if !with_user_db(state, move |db| Ok(connections::set_active(db, id)?)).await? {
        return Err(no_such_connection());
    }

    ai_config(state).await
}

/// Stores the API key of a connection, replacing an earlier one.
pub async fn store_key(state: &UiState, id: i64, api_key: String) -> Result<AiConfigView, UiError> {
    // The text is wiped by ApiKey::new, whether it is a key or not.
    let api_key = ApiKey::new(api_key).map_err(|error| UiError::Invalid(format!("the key was not stored: {error}")))?;

    if !with_user_db(state, move |db| Ok(connections::store_api_key(db, id, &api_key)?)).await? {
        return Err(no_such_connection());
    }

    ai_config(state).await
}

/// Deletes the API key of a connection.
pub async fn delete_key(state: &UiState, id: i64) -> Result<AiConfigView, UiError> {
    with_user_db(state, move |db| Ok(connections::remove_api_key(db, id)?)).await?;

    ai_config(state).await
}

/// Checks a stored connection with one short message.
pub async fn check_connection(state: &UiState, id: i64) -> Result<CheckView, UiError> {
    let (connection, api_key) = with_user_db(state, move |db| {
        let stored = connections::get(db, id)?.ok_or_else(no_such_connection)?;

        Ok((stored.connection, connections::api_key(db, id)?))
    })
    .await?;

    let connection = connection.check().map_err(UiError::Connection)?;

    Ok(CheckView::from(client::check(&connection, api_key.as_ref()).await))
}

/// The instruction for a chatbot to help fill `field`, with the form
/// as it is now (saved or not). Reads nothing stored; only for a
/// logged-in evaluator, like everything on the page.
pub fn field_instruction(state: &UiState, field: &str, form: ConnectionForm) -> Result<String, UiError> {
    state.session()?;

    let field = Field::from_name(field).ok_or_else(|| UiError::Invalid(format!("there is no field {field}")))?;

    Ok(field_help::instruction(field, &form.into_connection()?))
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "ai_config_actions_tests.rs"]
mod tests;
