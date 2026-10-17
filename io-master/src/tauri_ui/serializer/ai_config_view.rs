//! The AI connections as the tab AI of Settings shows and sends them,
//! and the report of a check.
//!
//! A connection travels in both directions in the same shape: the
//! page gets [`ConnectionView`], changes its fields and sends them
//! back as [`ConnectionForm`]. **The API key is never part of it**:
//! the view says only whether one is stored (`has_key`); a key is
//! sent alone, from a password field (`store_ai_key`).
//!
//! The presets (`db/ai/presets/read.rs`) come with the page as views of the
//! same shape: the page copies their fields into the form, as if the
//! evaluator had typed them.

use serde::{Deserialize, Serialize};

use crate::ai::api::checks::CheckReport;
use crate::ai::api::completions::ReplyEnd;
use crate::ai::api::connection::{API_KEY, AnswerMode, Connection};
use crate::db::ai::connection::rows::Stored;
use crate::db::ai::presets::read::Preset;
use crate::ai::api::completions::StopReason;

use super::super::error::UiError;

/// A connection as the page shows it.
#[derive(Debug, Serialize)]
pub struct ConnectionView {
    /// `None` for the template of a new connection.
    pub id: Option<i64>,
    pub name: String,
    pub url: String,
    pub model: String,
    pub ca_certificate: String,
    pub headers: String,
    pub body: String,
    pub message_template: String,
    pub user_role: String,
    pub model_role: String,
    pub system_role: String,
    pub system_prompt: String,
    /// `"stream"` or `"whole"`.
    pub answer: &'static str,
    pub text_path: String,
    pub stop_path: String,
    pub cut_reasons: String,
    pub refused_reasons: String,
    pub error_path: String,
    pub model_path: String,
    pub input_tokens_path: String,
    pub output_tokens_path: String,
    pub done_marker: String,
    /// Whether an API key is stored for it.
    pub has_key: bool,
    /// A key is stored, but no header uses `{{api_key}}`: the key is
    /// never sent (the page says so).
    pub key_unused: bool,
}

impl ConnectionView {
    fn new(id: Option<i64>, connection: Connection, has_key: bool) -> ConnectionView {
        let key_unused = has_key && !connection.headers.contains(API_KEY);

        ConnectionView {
            id,
            name: connection.name,
            url: connection.url,
            model: connection.model,
            ca_certificate: connection.ca_certificate,
            headers: connection.headers,
            body: connection.body,
            message_template: connection.message_template,
            user_role: connection.user_role,
            model_role: connection.model_role,
            system_role: connection.system_role,
            system_prompt: connection.system_prompt,
            answer: connection.answer.name(),
            text_path: connection.text_path,
            stop_path: connection.stop_path,
            cut_reasons: connection.cut_reasons,
            refused_reasons: connection.refused_reasons,
            error_path: connection.error_path,
            model_path: connection.model_path,
            input_tokens_path: connection.input_tokens_path,
            output_tokens_path: connection.output_tokens_path,
            done_marker: connection.done_marker,
            has_key,
            key_unused,
        }
    }
}

impl From<Stored> for ConnectionView {
    fn from(stored: Stored) -> ConnectionView {
        ConnectionView::new(Some(stored.id), stored.connection, stored.has_key)
    }
}

/// The tab AI of Settings.
#[derive(Debug, Serialize)]
pub struct AiConfigView {
    /// The connections, by name.
    pub connections: Vec<ConnectionView>,
    /// The connection the chat uses.
    pub active: Option<i64>,
    /// The starting point of a new connection.
    pub template: ConnectionView,
    /// The presets of well-known services.
    pub presets: Vec<PresetView>,
}

/// A preset as the page offers it.
#[derive(Debug, Serialize)]
pub struct PresetView {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    /// The fields it fills (`id` is `None`: nothing is stored).
    pub fields: ConnectionView,
}

impl From<Preset> for PresetView {
    fn from(preset: Preset) -> PresetView {
        PresetView {
            id: preset.id,
            title: preset.title,
            description: preset.description,
            fields: ConnectionView::new(None, preset.connection, false),
        }
    }
}

impl AiConfigView {
    pub fn new(connections: Vec<Stored>, active: Option<i64>) -> AiConfigView {
        AiConfigView {
            connections: connections.into_iter().map(ConnectionView::from).collect(),
            active,
            template: ConnectionView::new(None, Connection::template(), false),
            presets: crate::db::ai::presets::read::all().into_iter().map(PresetView::from).collect(),
        }
    }
}

/// A connection as the page sends it: the fields of
/// [`ConnectionView`] without `id`, `has_key` and `key_unused`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionForm {
    pub name: String,
    pub url: String,
    pub model: String,
    pub ca_certificate: String,
    pub headers: String,
    pub body: String,
    pub message_template: String,
    pub user_role: String,
    pub model_role: String,
    pub system_role: String,
    pub system_prompt: String,
    pub answer: String,
    pub text_path: String,
    pub stop_path: String,
    pub cut_reasons: String,
    pub refused_reasons: String,
    pub error_path: String,
    pub model_path: String,
    pub input_tokens_path: String,
    pub output_tokens_path: String,
    pub done_marker: String,
}

impl ConnectionForm {
    /// The connection of the core; only the kind of answer can be
    /// wrong here, every other rule is checked by the core.
    pub fn into_connection(self) -> Result<Connection, UiError> {
        let answer = AnswerMode::from_name(&self.answer)
            .ok_or_else(|| UiError::Invalid("the answer is either a stream or one JSON".to_owned()))?;

        Ok(Connection {
            name: self.name,
            url: self.url,
            model: self.model,
            ca_certificate: self.ca_certificate,
            headers: self.headers,
            body: self.body,
            message_template: self.message_template,
            user_role: self.user_role,
            model_role: self.model_role,
            system_role: self.system_role,
            system_prompt: self.system_prompt,
            answer,
            text_path: self.text_path,
            stop_path: self.stop_path,
            cut_reasons: self.cut_reasons,
            refused_reasons: self.refused_reasons,
            error_path: self.error_path,
            model_path: self.model_path,
            input_tokens_path: self.input_tokens_path,
            output_tokens_path: self.output_tokens_path,
            done_marker: self.done_marker,
        })
    }
}

/// The page after a connection was saved.
#[derive(Debug, Serialize)]
pub struct SavedView {
    pub config: AiConfigView,
    /// The connection saved (a new one gets its number here).
    pub id: i64,
    /// The sentence for the page.
    pub notice: String,
}

/// A header of the request of a check.
#[derive(Debug, Serialize)]
pub struct HeaderView {
    pub name: String,
    /// The value; the key masked.
    pub value: String,
}

/// The report of a check.
#[derive(Debug, Serialize)]
pub struct CheckView {
    /// Whether the connection works: a reply came and was read.
    pub works: bool,
    /// The sentence for the page.
    pub outcome: String,
    pub url: String,
    pub headers: Vec<HeaderView>,
    /// The body as it was sent, indented.
    pub body: String,
    /// The HTTP status; `None` when no answer came.
    pub status: Option<u16>,
    /// The beginning of the answer as it came; the key masked.
    pub raw: String,
    /// What the paths found.
    pub text: String,
    pub model: Option<String>,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

impl From<CheckReport> for CheckView {
    fn from(report: CheckReport) -> CheckView {
        let reply = report.reply;

        let (works, outcome) = match &reply.end {
            ReplyEnd::Finished(StopReason::Complete) if reply.text.is_empty() => (
                false,
                "An answer came, but the path to the text found no text in it. Compare the path with the raw answer."
                    .to_owned(),
            ),
            ReplyEnd::Finished(StopReason::Complete) => (true, "The connection works: the model answered.".to_owned()),
            ReplyEnd::Finished(StopReason::Cut) => (
                true,
                "The connection works; the reply was cut at its largest length (see the body).".to_owned(),
            ),
            ReplyEnd::Finished(StopReason::Refused) => {
                (true, "The connection works; the model declined the test message.".to_owned())
            }
            ReplyEnd::Cancelled => (false, "The check was stopped.".to_owned()),
            ReplyEnd::Failed(error) => (false, format!("The connection does not work: {error}.")),
        };

        CheckView {
            works,
            outcome,
            url: report.request.url,
            headers: report
                .request
                .headers
                .iter()
                .map(|(name, value)| HeaderView {
                    name: name.clone(),
                    value: value.as_str().to_owned(),
                })
                .collect(),
            body: serde_json::to_string_pretty(&report.request.body).unwrap_or_default(),
            status: report.status,
            raw: report.raw,
            text: reply.text,
            model: reply.model,
            input_tokens: reply.input_tokens,
            output_tokens: reply.output_tokens,
        }
    }
}
