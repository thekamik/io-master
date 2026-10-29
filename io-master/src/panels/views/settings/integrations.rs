//! The integrations of the settings of the AI, as the page calls them: the
//! list, a new one, one saved, the active one, its API key, the check of the
//! connection.
//!
//! ```text
//! Integrations.svelte ──integration_list()──────► the integrations, the active one, has a key ──► [IntegrationView]
//!                     ──integration_create(...)─► add_integration ──► id
//!                     ──integration_save(...)───► change_integration
//!                     ──integration_activate()──► activate_integration
//!                     ──integration_set_key()───► set_key            (the key goes in, never out)
//!                     ──integration_check(id)───► check_question ─► ask::send ─► ask::record ──► CheckView
//!                                                  (the lock held)   (released)   (the lock held; no agent,
//!                                                                                  so nothing is written)
//! ```

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::ai::ask::record;
use crate::db::ai::integration::read_integrations;
use crate::db::user::program_data::read_settings;
use crate::model::ai::integration::{Integration, Protocol};
use crate::model::ai::journal::Ending;
use crate::panels::logic::settings::integrations::{
    activate_integration, add_integration, change_integration, check_question, has_key, set_key,
};
use crate::panels::views::answer::send_drained;
use crate::panels::views::error::ViewError;
use crate::panels::views::opened::database;
use crate::session::state::Session;

/// The longest text of a field of an integration from the page (a name, an
/// address, a model), in bytes.
const MAX_FIELD: usize = 2048;

/// The longest CA certificate from the page, in bytes: a chain of a few
/// certificates.
const MAX_CERTIFICATE: usize = 64 * 1024;

/// An integration, as the page lists it. The key is never in it: only
/// whether there is one.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationView {
    id: i64,
    name: String,
    url: String,
    model: String,
    /// "openai_chat", "anthropic_messages".
    protocol: &'static str,
    /// Empty: the trusted certificates of the system.
    ca_certificate: String,
    context_size: u32,
    has_key: bool,
    active: bool,
}

/// What the check of an integration found.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckView {
    /// "complete", "cut", "stopped", "refused", "failed".
    ending: &'static str,
    text: String,
    model: String,
    /// Why it failed; `None` when it did not.
    error: Option<String>,
    input_tokens: u32,
    output_tokens: u32,
}

/// The name of a protocol on the page.
///
/// Called by: [`integration_list`].
fn protocol_name(protocol: Protocol) -> &'static str {
    match protocol {
        Protocol::OpenAiChat => "openai_chat",
        Protocol::AnthropicMessages => "anthropic_messages",
    }
}

/// The protocol of its name on the page; a name the page never sends is
/// refused.
///
/// Called by: [`integration_of`].
fn protocol_of(name: &str) -> Result<Protocol, ViewError> {
    match name {
        "openai_chat" => Ok(Protocol::OpenAiChat),
        "anthropic_messages" => Ok(Protocol::AnthropicMessages),
        _ => Err(ViewError::Invalid(format!("no protocol \"{name}\""))),
    }
}

/// The name of an ending on the page.
///
/// Called by: [`integration_check`]; `journal.rs`.
pub fn ending_name(ending: Ending) -> &'static str {
    match ending {
        Ending::Complete => "complete",
        Ending::Cut => "cut",
        Ending::Stopped => "stopped",
        Ending::Refused => "refused",
        Ending::Failed => "failed",
    }
}

/// The fields of an integration from the page, checked for their size, as
/// the model.
///
/// Called by: [`integration_create`], [`integration_save`].
fn integration_of(fields: IntegrationFields) -> Result<Integration, ViewError> {
    for text in [&fields.name, &fields.url, &fields.model] {
        ensure!(text.len() <= MAX_FIELD, ViewError::Invalid(format!("a field over {MAX_FIELD} bytes")));
    }

    ensure!(
        fields.ca_certificate.len() <= MAX_CERTIFICATE,
        ViewError::Invalid(format!("a CA certificate over {MAX_CERTIFICATE} bytes"))
    );

    Ok(Integration {
        id: fields.id,
        name: fields.name,
        url: fields.url,
        model: fields.model,
        protocol: protocol_of(&fields.protocol)?,
        ca_certificate: Some(fields.ca_certificate),
        context_size: fields.context_size,
    })
}

/// The fields of an integration as the page sends them (`id` 0 for a new
/// one).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationFields {
    id: i64,
    name: String,
    url: String,
    model: String,
    protocol: String,
    ca_certificate: String,
    context_size: u32,
}

/// Every integration of the user, the active one marked, whether each has a
/// key.
///
/// Called by: `Integrations.svelte`, when the tab opens and after a change.
/// Calls: `read_integrations`, `read_settings`, `has_key`.
#[tauri::command]
pub fn integration_list(session: State<'_, Session>) -> Result<Vec<IntegrationView>, ViewError> {
    let opened = session.lock()?;
    let db = database(&opened)?;
    let active = read_settings(db)?.active_integration;
    let mut integrations = Vec::new();

    for integration in read_integrations(db)? {
        integrations.push(IntegrationView {
            has_key: has_key(db, integration.id)?,
            active: active == Some(integration.id),
            id: integration.id,
            name: integration.name,
            url: integration.url,
            model: integration.model,
            protocol: protocol_name(integration.protocol),
            ca_certificate: integration.ca_certificate.unwrap_or_default(),
            context_size: integration.context_size,
        });
    }

    Ok(integrations)
}

/// Stores a new integration; gives its id.
///
/// Called by: `Integrations.svelte`, "Add".
/// Calls: [`integration_of`], `add_integration`.
#[tauri::command]
pub fn integration_create(session: State<'_, Session>, fields: IntegrationFields) -> Result<i64, ViewError> {
    let integration = integration_of(fields)?;
    let opened = session.lock()?;

    Ok(add_integration(database(&opened)?, &integration)?)
}

/// Saves the data of the integration `fields.id`.
///
/// Called by: `Integrations.svelte`, "Save".
/// Calls: [`integration_of`], `change_integration`.
#[tauri::command]
pub fn integration_save(session: State<'_, Session>, fields: IntegrationFields) -> Result<(), ViewError> {
    let integration = integration_of(fields)?;
    let opened = session.lock()?;

    Ok(change_integration(database(&opened)?, &integration)?)
}

/// Makes the integration `id` the active one.
///
/// Called by: `Integrations.svelte`, "Make active".
/// Calls: `activate_integration`.
#[tauri::command]
pub fn integration_activate(session: State<'_, Session>, id: i64) -> Result<(), ViewError> {
    let opened = session.lock()?;

    Ok(activate_integration(database(&opened)?, id)?)
}

/// Stores the API key of the integration `id`. The key goes in only: no
/// command gives it back.
///
/// Called by: `Integrations.svelte`, "Store the key".
/// Calls: `set_key`.
#[tauri::command]
pub fn integration_set_key(session: State<'_, Session>, id: i64, key: String) -> Result<(), ViewError> {
    let opened = session.lock()?;

    Ok(set_key(database(&opened)?, id, &key)?)
}

/// Checks the integration `id` with one short question. It has no agent, so
/// it is not kept in a journal. The lock of the session is released while
/// the answer is awaited, so the window stays usable.
///
/// Called by: `Integrations.svelte`, "Check the connection".
/// Calls: `check_question`, `answer::send_drained`, `ask::record`.
#[tauri::command]
pub async fn integration_check(session: State<'_, Session>, id: i64) -> Result<CheckView, ViewError> {
    let prepared = {
        let opened = session.lock()?;

        check_question(database(&opened)?, id)?
    };
    let sent = send_drained(&prepared).await;
    let opened = session.lock()?;
    let answer = record(database(&opened)?, prepared, sent)?;

    Ok(CheckView {
        ending: ending_name(answer.ending),
        text: answer.text,
        model: answer.model,
        error: answer.error.map(|error| error.to_string()),
        input_tokens: answer.input_tokens,
        output_tokens: answer.output_tokens,
    })
}
