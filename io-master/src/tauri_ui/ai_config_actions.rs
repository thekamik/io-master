//! The tab AI of Settings: the AI connections of the evaluator, their
//! API keys, the active connection, and the check of a connection.
//!
//! **A key is never sent back to the page**: the page learns only
//! whether one is stored. Saving a connection needs no answer of the
//! service (a local model may be switched off while it is set up);
//! "Check connection" ([`check_connection`]) asks the service and
//! reports what happened (and writes the check into the journal of the
//! AI). "AI help" ([`field_instruction`]) gives the instruction for a
//! chatbot that helps to fill one field.

use crate::ai::api::api_key::ApiKey;
use crate::ai::api::checks;
use crate::ai::api::connection::Field;
use crate::db::ai::connection::{create, delete, read, update};
use crate::db::ai::presets::field_help;

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
    with_user_db(state, |db| Ok(AiConfigView::new(read::list(db)?, read::active(db)?))).await
}

/// Saves a connection (a new one when `id` is `None`).
pub async fn save_connection(state: &UiState, id: Option<i64>, form: ConnectionForm) -> Result<SavedView, UiError> {
    let connection = form.into_connection()?;

    with_user_db(state, move |db| {
        let saved = match id {
            None => create::connection(db, &connection)?,
            Some(id) => update::connection(db, id, &connection)?,
        };

        let notice = if saved.key_removed {
            "The connection was saved. Its API key was removed, because the address changed; enter the key again \
             if the new address needs it."
        } else {
            "The connection was saved."
        };

        Ok(SavedView {
            config: AiConfigView::new(read::list(db)?, read::active(db)?),
            id: saved.id,
            notice: notice.to_owned(),
        })
    })
    .await
}

/// Deletes a connection with its API key.
pub async fn delete_connection(state: &UiState, id: i64) -> Result<AiConfigView, UiError> {
    if !with_user_db(state, move |db| Ok(delete::connection(db, id)?)).await? {
        return Err(no_such_connection());
    }

    ai_config(state).await
}

/// Makes a connection the one the chat uses.
pub async fn activate_connection(state: &UiState, id: i64) -> Result<AiConfigView, UiError> {
    if !with_user_db(state, move |db| Ok(update::active(db, id)?)).await? {
        return Err(no_such_connection());
    }

    ai_config(state).await
}

/// Stores the API key of a connection, replacing an earlier one.
pub async fn store_key(state: &UiState, id: i64, api_key: String) -> Result<AiConfigView, UiError> {
    // The text is wiped by ApiKey::new, whether it is a key or not.
    let api_key = ApiKey::new(api_key).map_err(|error| UiError::Invalid(format!("the key was not stored: {error}")))?;

    if !with_user_db(state, move |db| Ok(update::api_key(db, id, &api_key)?)).await? {
        return Err(no_such_connection());
    }

    ai_config(state).await
}

/// Deletes the API key of a connection.
pub async fn delete_key(state: &UiState, id: i64) -> Result<AiConfigView, UiError> {
    with_user_db(state, move |db| Ok(delete::api_key(db, id)?)).await?;

    ai_config(state).await
}

/// Checks a stored connection with one short message.
pub async fn check_connection(state: &UiState, id: i64) -> Result<CheckView, UiError> {
    let model = with_user_db(state, move |db| {
        read::get(db, id)?.ok_or_else(no_such_connection)?;

        Ok(checks::target(db, id)?)
    })
    .await?;

    let report = checks::create(&model).await;

    // The check is a question to the AI: it goes into the journal. A
    // failure to write it does not hide the report.
    let journal = with_user_db(state, {
        let request = report.request.body.to_string();
        let reply = report.reply.clone();
        move |db| Ok(checks::record(db, &model, &request, &reply)?)
    })
    .await;

    let mut view = CheckView::from(report);

    if let Err(error) = journal {
        view.outcome = format!("{} The check was not written into the journal of the AI: {error}.", view.outcome);
    }

    Ok(view)
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
