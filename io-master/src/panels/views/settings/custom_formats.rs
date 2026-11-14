//! The custom formats of the settings of the AI, as the page calls them:
//! the list, where a new one starts, a new one, one saved, one tried
//! without a connection.
//!
//! ```text
//! CustomFormats.svelte ──custom_format_list()──────► read_custom_formats ──► [CustomFormatView]
//!                      ──custom_format_new()───────► new_format: the smallest shapes that work
//!                      ──custom_format_create(...)─► add_format ──► id
//!                      ──custom_format_save(...)───► change_format
//!                      ──custom_format_try(...)────► tried: the request filled, a sample of an answer read
//!                                                     ──► TrialView (nothing stored, nothing sent)
//! ```
//!
//! The connection itself is tried with the check of an integration that
//! names the format (`panels/views/settings/integrations.rs`:
//! `integration_check`).

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::objects::ai::custom_format::custom_format_ai::tried;
use crate::objects::ai::custom_format::custom_format_model::{CustomFormat, ResponseKind};
use crate::objects::ai::custom_format::custom_format_rules::{add_format, change_format, new_format};
use crate::objects::ai::custom_format::custom_format_store::{MAX_FORMAT_JSON, read_custom_formats};
use crate::panels::session::Session;
use crate::panels::views::error::ViewError;
use crate::panels::views::opened::database;

/// The longest name, header of the key and text before the key from the
/// page, in bytes.
const MAX_FIELD: usize = 2048;

/// A custom format, as the page shows and sends it, both its JSON texts
/// whole.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomFormatView {
    /// 0 for a format not stored yet.
    id: i64,
    name: String,
    request: String,
    response: String,
    /// `"whole"`, `"events"`.
    response_kind: String,
    key_header: String,
    key_prefix: String,
}

/// What a trial of a format gives.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrialView {
    /// The request as it would be sent, filled with a sample of a question.
    request: String,
    /// Whether a sample of an answer was given and read.
    read: bool,
    text: String,
    model: String,
    input_tokens: u32,
    output_tokens: u32,
}

/// The name of a kind of response for the page (`ResponseKind::Whole` →
/// `"whole"`).
///
/// Called by: [`view_of`].
fn kind_name(kind: ResponseKind) -> &'static str {
    match kind {
        ResponseKind::Whole => "whole",
        ResponseKind::Events => "events",
    }
}

/// The view of the format `format`.
///
/// Called by: [`custom_format_list`], [`custom_format_new`].
/// Calls: [`kind_name`].
fn view_of(format: CustomFormat) -> CustomFormatView {
    CustomFormatView {
        id: format.id,
        name: format.name,
        request: format.request,
        response: format.response,
        response_kind: kind_name(format.response_kind).to_owned(),
        key_header: format.key_header,
        key_prefix: format.key_prefix,
    }
}

/// The format the page sent, its sizes checked at this entry; its content
/// is checked by the rules.
///
/// Input: `fields`, a format as the page sends it.
/// Output: the format; `ViewError::Invalid` for a field over its limit or a
/// kind of response the page never sends.
///
/// Called by: [`custom_format_create`], [`custom_format_save`], [`custom_format_try`].
fn format_of(fields: CustomFormatView) -> Result<CustomFormat, ViewError> {
    for text in [&fields.name, &fields.key_header, &fields.key_prefix] {
        ensure!(text.len() <= MAX_FIELD, ViewError::Invalid(format!("a field over {MAX_FIELD} bytes")));
    }

    ensure!(
        fields.request.len() <= MAX_FORMAT_JSON && fields.response.len() <= MAX_FORMAT_JSON,
        ViewError::Invalid(format!("a JSON over {MAX_FORMAT_JSON} bytes"))
    );

    let response_kind = match fields.response_kind.as_str() {
        "whole" => ResponseKind::Whole,
        "events" => ResponseKind::Events,
        other => return Err(ViewError::Invalid(format!("no kind of response \"{other}\""))),
    };

    Ok(CustomFormat {
        id: fields.id,
        name: fields.name,
        request: fields.request,
        response: fields.response,
        response_kind,
        key_header: fields.key_header,
        key_prefix: fields.key_prefix,
    })
}

/// Every custom format of the user, by its name.
///
/// Input: `session`, the session of the window, someone logged in.
/// Output: the formats (`[{"id": 2, "name": "The institute", "request":
/// "{...}", "responseKind": "whole", ...}]`); `ViewError::NotLoggedIn` when
/// nobody is logged in, an error of the database.
///
/// Called by: `ui/src/lib/api/settings.ts`: `customFormatList`, from
/// `ui/src/views/settings/CustomFormats.svelte` (when the tab opens and after a change) and
/// `ui/src/views/settings/IntegrationForm.svelte` (the format of an integration).
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/ai/custom_format/custom_format_store.rs`: `read_custom_formats`; [`view_of`].
#[tauri::command(async)]
pub fn custom_format_list(session: State<'_, Session>) -> Result<Vec<CustomFormatView>, ViewError> {
    let opened = session.lock()?;
    let mut formats = Vec::new();

    for format in read_custom_formats(database(opened.as_ref())?)? {
        formats.push(view_of(format));
    }

    Ok(formats)
}

/// Where a new custom format starts: nothing read.
///
/// Called by: `ui/src/lib/api/settings.ts`: `customFormatNew`, from
/// `ui/src/views/settings/CustomFormats.svelte`, "A new format".
/// Calls: `objects/ai/custom_format/custom_format_rules.rs`: `new_format`; [`view_of`].
#[tauri::command(async)]
pub fn custom_format_new() -> CustomFormatView {
    view_of(new_format())
}

/// Stores a new custom format.
///
/// Input: `session`, the session of the window, someone logged in;
/// `fields`, the format typed, its id 0.
/// Output: the id of the new format (`2`); `ViewError::NotLoggedIn`, a
/// refusal of the rules with its reason (a request without a place for the
/// question, a response without `<response>`), an error of the database.
///
/// Called by: `ui/src/lib/api/settings.ts`: `customFormatCreate`, from
/// `ui/src/views/settings/CustomFormats.svelte`, "Add the format".
/// Calls: [`format_of`]; `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/ai/custom_format/custom_format_rules.rs`: `add_format`.
#[tauri::command(async)]
pub fn custom_format_create(session: State<'_, Session>, fields: CustomFormatView) -> Result<i64, ViewError> {
    let format = format_of(fields)?;
    let opened = session.lock()?;

    Ok(add_format(database(opened.as_ref())?, &format)?)
}

/// Writes a stored custom format anew.
///
/// Input: `session`, the session of the window, someone logged in;
/// `fields`, the format as changed, its id that of a stored one.
/// Output: nothing; the errors of [`custom_format_create`], an error of the
/// database for a format missing.
///
/// Called by: `ui/src/lib/api/settings.ts`: `customFormatSave`, from
/// `ui/src/views/settings/CustomFormats.svelte`, "Save".
/// Calls: [`format_of`]; `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/ai/custom_format/custom_format_rules.rs`: `change_format`.
#[tauri::command(async)]
pub fn custom_format_save(session: State<'_, Session>, fields: CustomFormatView) -> Result<(), ViewError> {
    let format = format_of(fields)?;
    let opened = session.lock()?;

    Ok(change_format(database(opened.as_ref())?, &format)?)
}

/// Tries a format as typed, without storing it and without any connection:
/// its request filled with a sample of a question, and a sample of an answer
/// read as the answer of the service would be.
///
/// Input: `session`, the session of the window, someone logged in;
/// `fields`, the format as typed; `sample`, an answer of the service copied
/// from its documentation (one event of it for a stream), or empty.
/// Output: the trial (`{"request": "{\n  \"prompt\": \"The question.\"\n}",
/// "read": true, "text": "PASS", ...}`); `ViewError::NotLoggedIn`, a refusal
/// of the format or of the reading with its reason.
///
/// Called by: `ui/src/lib/api/settings.ts`: `customFormatTry`, from
/// `ui/src/views/settings/CustomFormats.svelte`, "Try".
/// Calls: [`format_of`]; `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/ai/custom_format/custom_format_ai.rs`: `tried`.
#[tauri::command(async)]
pub fn custom_format_try(
    session: State<'_, Session>,
    fields: CustomFormatView,
    sample: String,
) -> Result<TrialView, ViewError> {
    ensure!(sample.len() <= MAX_FORMAT_JSON, ViewError::Invalid(format!("a sample over {MAX_FORMAT_JSON} bytes")));

    let format = format_of(fields)?;

    // Only someone logged in works with the formats; the trial reads nothing.
    database(session.lock()?.as_ref())?;

    let (request, reading) = tried(&format, &sample)?;
    let read = reading.is_some();
    let reading = reading.unwrap_or_default();

    Ok(TrialView {
        request,
        read,
        text: reading.text,
        model: reading.model.unwrap_or_default(),
        input_tokens: reading.input_tokens,
        output_tokens: reading.output_tokens,
    })
}
