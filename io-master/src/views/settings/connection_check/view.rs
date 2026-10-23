//! The report of a check of a connection, as the page gets it; the key
//! masked everywhere (`ai/api/checks.rs`).

use serde::Serialize;

use crate::ai::api::checks::CheckReport;
use crate::ai::api::completions::{ReplyEnd, StopReason};
use crate::db::error::DbError;

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

impl CheckView {
    /// The report of a check; `journal` is why the check could not be
    /// written into the journal of the AI, if it could not (the report
    /// is shown all the same).
    pub fn new(report: CheckReport, journal: Option<DbError>) -> CheckView {
        let (works, mut outcome) = outcome(&report.reply.end, report.reply.text.is_empty());

        if let Some(error) = journal {
            outcome = format!("{outcome} The check was not written into the journal of the AI: {error}.");
        }

        let mut headers = Vec::with_capacity(report.request.headers.len());

        for (name, value) in &report.request.headers {
            headers.push(HeaderView { name: name.clone(), value: value.as_str().to_owned() });
        }

        CheckView {
            works,
            outcome,
            url: report.request.url,
            headers,
            body: indented(&report.request.body),
            status: report.status,
            raw: report.raw,
            text: report.reply.text,
            model: report.reply.model,
            input_tokens: report.reply.input_tokens,
            output_tokens: report.reply.output_tokens,
        }
    }
}

/// Whether the connection works, and the sentence for the page.
fn outcome(end: &ReplyEnd, no_text: bool) -> (bool, String) {
    match end {
        ReplyEnd::Finished(StopReason::Complete) if no_text => (
            false,
            "An answer came, but the path to the text found no text in it. Compare the path with the raw answer."
                .to_owned(),
        ),
        ReplyEnd::Finished(StopReason::Complete) => (true, "The connection works: the model answered.".to_owned()),
        ReplyEnd::Finished(StopReason::Cut) => {
            (true, "The connection works; the reply was cut at its largest length (see the body).".to_owned())
        }
        ReplyEnd::Finished(StopReason::Refused) => {
            (true, "The connection works; the model declined the test message.".to_owned())
        }
        ReplyEnd::Cancelled => (false, "The check was stopped.".to_owned()),
        ReplyEnd::Failed(error) => (false, format!("The connection does not work: {error}.")),
    }
}

/// The body of the request, indented for reading.
fn indented(body: &serde_json::Value) -> String {
    match serde_json::to_string_pretty(body) {
        Ok(text) => text,
        Err(error) => format!("(the body cannot be shown: {error})"),
    }
}
