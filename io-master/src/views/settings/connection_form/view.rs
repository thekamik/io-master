//! The data of the connection form, as the page gets it and sends it.
//!
//! A connection travels in both directions in the same shape: the page
//! gets [`ConnectionView`], changes its fields and sends them back as
//! [`ConnectionForm`]. **The API key is never part of it**: it has an
//! element of its own (`api_key/`). The presets (`ai/presets.rs`) come as
//! views of the same shape: the page copies their fields into the form.

use serde::{Deserialize, Serialize};

use crate::ai::presets::Preset;
use crate::app::error::AppError;
use crate::model::ai::{AnswerMode, Connection};

/// A connection as the page shows it.
#[derive(Debug, Serialize)]
pub struct ConnectionView {
    /// `None` for the template of a new connection and for a preset.
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
}

impl ConnectionView {
    pub fn new(id: Option<i64>, connection: Connection) -> ConnectionView {
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
        }
    }
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

/// What a new connection starts from.
#[derive(Debug, Serialize)]
pub struct NewConnectionView {
    /// The starting point of a new connection.
    pub template: ConnectionView,
    /// The presets of well-known services.
    pub presets: Vec<PresetView>,
}

impl NewConnectionView {
    pub fn new(presets: Vec<Preset>) -> NewConnectionView {
        let mut views = Vec::with_capacity(presets.len());

        for preset in presets {
            views.push(PresetView {
                id: preset.id,
                title: preset.title,
                description: preset.description,
                fields: ConnectionView::new(None, preset.connection),
            });
        }

        NewConnectionView { template: ConnectionView::new(None, Connection::template()), presets: views }
    }
}

/// What saving did.
#[derive(Debug, Serialize)]
pub struct SavedView {
    /// The connection saved (a new one gets its number here).
    pub id: i64,
    /// The sentence for the page.
    pub notice: &'static str,
}

/// A connection as the page sends it: the fields of [`ConnectionView`]
/// without `id`. A field the page does not know is refused.
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
    /// The connection of the core; only the kind of answer can be wrong
    /// here, every other rule is checked by the core (`model/ai.rs`).
    pub fn into_connection(self) -> Result<Connection, AppError> {
        let Some(answer) = AnswerMode::from_name(&self.answer) else {
            return Err(AppError::Invalid("the answer is either a stream or one JSON".to_owned()));
        };

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
