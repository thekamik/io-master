//! The integrations of the settings of the AI, as the page calls them: the
//! list, where a new one starts and the limits of its fields, a new one, one
//! saved, the active one, its API key, the check of the connection.
//!
//! ```text
//! IntegrationList.svelte ──integration_list()──────► the integrations, the active one, has a key ──► [IntegrationView]
//!                        ──integration_activate()──► activate_integration
//! IntegrationForm.svelte ──integration_form()──────► the start of a new one, the limits ──► IntegrationFormView
//!                        ──integration_create(...)─► add_integration ──► id
//!                        ──integration_save(...)───► change_integration
//!                        ──integration_set_key()───► set_key            (the key goes in, never out)
//!                        ──integration_check(id)───► check_question ─► sending::send_whole ─► journal::record
//!                                                    (the lock held)   (released)   (the lock held; no agent,
//!                                                                                    so nothing is written) ──► CheckView
//! ```

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::objects::ai::api_key::api_key_rules::{has_key, set_key};
use crate::objects::ai::integration::integration_ai::check::check_question;
use crate::objects::ai::integration::integration_ai::question::context_warning;
use crate::objects::ai::integration::integration_ai::sending::send_whole;
use crate::objects::ai::integration::integration_model::{Integration, Protocol};
use crate::objects::ai::integration::integration_rules::settings::{
    MIN_CONTEXT, NEW_CHARACTERS_PER_TOKEN, NEW_CONNECT_TIMEOUT_SECONDS, NEW_STREAM_SILENCE_SECONDS,
    activate_integration, add_integration, change_integration,
};
use crate::objects::ai::integration::integration_store::{
    MAX_CHARACTERS_PER_TOKEN, MAX_CONNECT_TIMEOUT_SECONDS, MAX_STREAM_SILENCE_SECONDS, read_integration,
    read_integrations,
};
use crate::objects::ai::journal::journal_ai::record;
use crate::objects::ai::journal::journal_model::Ending;
use crate::objects::user::program_data::program_data_store::read_settings;
use crate::panels::session::Session;
use crate::panels::views::error::ViewError;
use crate::panels::views::opened::database;

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
    /// `"openai_chat"`, `"anthropic_messages"`.
    protocol: &'static str,
    /// Empty: the trusted certificates of the system.
    ca_certificate: String,
    context_size: u32,
    characters_per_token: u32,
    connect_timeout_seconds: u32,
    stream_silence_seconds: u32,
    /// Why the context is too small for an ST of 100 pages; `None` when it
    /// is not.
    context_warning: Option<String>,
    has_key: bool,
    active: bool,
}

/// Where a new integration starts and the limits of the fields of an
/// integration, as the program gives them (user, 2026-11-02: the page holds
/// no number of its own).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationFormView {
    characters_per_token: u32,
    connect_timeout_seconds: u32,
    stream_silence_seconds: u32,
    min_context: u32,
    max_characters_per_token: u32,
    max_connect_timeout_seconds: u32,
    max_stream_silence_seconds: u32,
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
    /// Why the context of the integration is too small for an ST of 100
    /// pages; `None` when it is not.
    context_warning: Option<String>,
}

/// The name of a protocol on the page.
///
/// Input: `protocol`, a protocol.
/// Output: its name on the page (`"openai"`).
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
/// Input: `name`, the name of a protocol the page sends (`"anthropic"`).
/// Output: the protocol; `ViewError::Invalid` for another name.
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
/// Input: `ending`, how an answer ended.
/// Output: its name on the page (`"complete"`).
///
/// Called by: [`integration_check`]; `panels/views/agents/journal.rs`.
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
/// Input: `fields`, the fields of the form of the page.
/// Output: the integration (`Integration { id: 0, name: "Ollama", .. }`);
/// `ViewError::Invalid` for a field over its size or a protocol the page
/// never sends.
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
        characters_per_token: fields.characters_per_token,
        connect_timeout_seconds: fields.connect_timeout_seconds,
        stream_silence_seconds: fields.stream_silence_seconds,
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
    characters_per_token: u32,
    connect_timeout_seconds: u32,
    stream_silence_seconds: u32,
}

/// Every integration of the user, the active one marked, whether each has a
/// key.
///
/// Input: `session`, the session of the window, someone logged in.
/// Output: the integrations, each with the warning about a context too small
/// for an ST of 100 pages (`[{"id": 1, "name": "Ollama", "active": true,
/// "hasKey": false, "contextWarning": null, ...}]`); `ViewError::NotLoggedIn`
/// when nobody is logged in, an error of the database.
///
/// Called by: `ui/src/lib/api/settings.ts`: `integrationList`, from `ui/src/views/settings/IntegrationList.svelte`,
/// when the tab opens and after a change; `ui/src/views/agents/AgentList.svelte` (the integration of a new agent).
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/ai/integration/integration_store.rs`: `read_integrations`;
/// `objects/user/program_data/program_data_store.rs`: `read_settings`; `objects/ai/api_key/api_key_rules.rs`:
/// `has_key`.
#[tauri::command]
pub fn integration_list(session: State<'_, Session>) -> Result<Vec<IntegrationView>, ViewError> {
    let opened = session.lock()?;
    let db = database(opened.as_ref())?;
    let active = read_settings(db)?.active_integration;
    let mut integrations = Vec::new();

    for integration in read_integrations(db)? {
        integrations.push(IntegrationView {
            context_warning: context_warning(&integration),
            has_key: has_key(db, integration.id)?,
            active: active == Some(integration.id),
            id: integration.id,
            name: integration.name,
            url: integration.url,
            model: integration.model,
            protocol: protocol_name(integration.protocol),
            ca_certificate: integration.ca_certificate.unwrap_or_default(),
            context_size: integration.context_size,
            characters_per_token: integration.characters_per_token,
            connect_timeout_seconds: integration.connect_timeout_seconds,
            stream_silence_seconds: integration.stream_silence_seconds,
        });
    }

    Ok(integrations)
}

/// Where a new integration starts and the limits of the fields of an
/// integration: the constants of the program, nothing read.
///
/// Output: the view (`{"charactersPerToken": 3, "connectTimeoutSeconds": 15,
/// "streamSilenceSeconds": 300, "minContext": 512, ...}`).
///
/// Called by: `ui/src/lib/api/settings.ts`: `integrationForm`, from `ui/src/views/settings/IntegrationForm.svelte`,
/// when the form opens.
#[tauri::command]
pub fn integration_form() -> IntegrationFormView {
    IntegrationFormView {
        characters_per_token: NEW_CHARACTERS_PER_TOKEN,
        connect_timeout_seconds: NEW_CONNECT_TIMEOUT_SECONDS,
        stream_silence_seconds: NEW_STREAM_SILENCE_SECONDS,
        min_context: MIN_CONTEXT,
        max_characters_per_token: MAX_CHARACTERS_PER_TOKEN,
        max_connect_timeout_seconds: MAX_CONNECT_TIMEOUT_SECONDS,
        max_stream_silence_seconds: MAX_STREAM_SILENCE_SECONDS,
    }
}

/// Stores a new integration; gives its id.
///
/// Input: `session`, the session of the window, someone logged in; `fields`,
/// the fields of the form.
/// Output: the id of the new integration (`2`); `ViewError::NotLoggedIn` when
/// nobody is logged in, the refusals of [`integration_of`] and of the
/// integration.
///
/// Called by: `ui/src/lib/api/settings.ts`: `integrationCreate`, from
/// `ui/src/views/settings/IntegrationForm.svelte`, "Add the integration".
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`; [`integration_of`];
/// `objects/ai/integration/integration_rules/settings.rs`: `add_integration`.
#[tauri::command]
pub fn integration_create(session: State<'_, Session>, fields: IntegrationFields) -> Result<i64, ViewError> {
    let integration = integration_of(fields)?;
    let opened = session.lock()?;

    Ok(add_integration(database(opened.as_ref())?, &integration)?)
}

/// Saves the data of the integration `fields.id`.
///
/// Input: `session`, the session of the window, someone logged in; `fields`,
/// the fields of the form, its id that of an integration.
/// Output: nothing; `ViewError::NotLoggedIn` when nobody is logged in, the
/// refusals of [`integration_of`] and of the integration.
///
/// Called by: `ui/src/lib/api/settings.ts`: `integrationSave`, from `ui/src/views/settings/IntegrationForm.svelte`,
/// "Save".
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`; [`integration_of`];
/// `objects/ai/integration/integration_rules/settings.rs`: `change_integration`.
#[tauri::command]
pub fn integration_save(session: State<'_, Session>, fields: IntegrationFields) -> Result<(), ViewError> {
    let integration = integration_of(fields)?;
    let opened = session.lock()?;

    Ok(change_integration(database(opened.as_ref())?, &integration)?)
}

/// Makes the integration `id` the active one.
///
/// Called by: `ui/src/lib/api/settings.ts`: `integrationActivate`, from
/// `ui/src/views/settings/IntegrationList.svelte`, "Make active".
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/ai/integration/integration_rules/settings.rs`: `activate_integration`.
#[tauri::command]
pub fn integration_activate(session: State<'_, Session>, id: i64) -> Result<(), ViewError> {
    let opened = session.lock()?;

    Ok(activate_integration(database(opened.as_ref())?, id)?)
}

/// Stores the API key of the integration `id`. The key goes in only: no
/// command gives it back.
///
/// Called by: `ui/src/lib/api/settings.ts`: `integrationSetKey`, from
/// `ui/src/views/settings/IntegrationForm.svelte`, "Store the key".
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/ai/api_key/api_key_rules.rs`: `set_key`.
#[tauri::command]
pub fn integration_set_key(session: State<'_, Session>, id: i64, key: String) -> Result<(), ViewError> {
    let opened = session.lock()?;

    Ok(set_key(database(opened.as_ref())?, id, &key)?)
}

/// Checks the integration `id` with one short question. It has no agent, so
/// it is not kept in a journal. The lock of the session is released while
/// the answer is awaited, so the window stays usable.
///
/// Input: `session`, the session of the window, someone logged in; `id`, the id
/// of an integration.
/// Output: the answer to the question of the check, with the warning about a
/// context too small for an ST of 100 pages (`{"text": "OK", "ending":
/// "complete", "error": null, "contextWarning": "The context of ..."}`);
/// `ViewError::NotLoggedIn` when nobody is logged in, the refusals of the
/// question, an error of the session or the database.
///
/// Called by: `ui/src/lib/api/settings.ts`: `integrationCheck`, from
/// `ui/src/views/settings/IntegrationForm.svelte`, "Check the connection".
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/ai/integration/integration_ai/check.rs`: `check_question`;
/// `objects/ai/integration/integration_ai/sending.rs`: `send_whole`; `objects/ai/journal/journal_ai.rs`: `record`.
#[tauri::command]
pub async fn integration_check(session: State<'_, Session>, id: i64) -> Result<CheckView, ViewError> {
    let (prepared, warning) = {
        let opened = session.lock()?;
        let db = database(opened.as_ref())?;

        (check_question(db, id)?, context_warning(&read_integration(db, id)?))
    };
    let sent = send_whole(&prepared).await;
    let opened = session.lock()?;
    let answer = record(database(opened.as_ref())?, prepared, sent)?;

    Ok(CheckView {
        ending: ending_name(answer.ending),
        text: answer.text,
        model: answer.model,
        error: answer.error.map(|error| error.to_string()),
        input_tokens: answer.input_tokens,
        output_tokens: answer.output_tokens,
        context_warning: warning,
    })
}
