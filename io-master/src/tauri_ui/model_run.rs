//! One question to the AI model of the active connection, for the long
//! works of the test projects: the analysis of a chapter
//! (`scheme_actions.rs`) and the reading of a table of contents
//! (`project_actions.rs`).
//!
//! The answer is not shown as it is written; only its length is
//! reported, so that the user sees the model working. "Stop" (the
//! signal of `UiState::start_work`) ends the request at once.

use tokio::sync::{mpsc, watch};

use crate::ai::api_key::ApiKey;
use crate::ai::client::{self, ReplyEnd};
use crate::ai::connection::Checked;
use crate::ai::connections;
use crate::ai::request::{Speaker, Turn};
use crate::db::user_db::UserDb;

use super::error::UiError;

/// The active AI connection and its key, read once before a long work.
pub struct Model {
    pub connection: Checked,
    pub api_key: Option<ApiKey>,
}

/// The active connection of the evaluator, checked; an error the page
/// can show when none is chosen or its key is missing.
pub fn active_model(db: &UserDb) -> Result<Model, UiError> {
    let id = connections::active(db)?
        .ok_or_else(|| UiError::Invalid("no AI connection is chosen; set one up in Settings, tab AI".to_owned()))?;
    let stored = connections::get(db, id)?.ok_or_else(|| UiError::Invalid("the AI connection is gone".to_owned()))?;
    let connection = stored.connection.check().map_err(UiError::Connection)?;
    let api_key = connections::api_key(db, id)?;

    if connection.needs_key() && api_key.is_none() {
        return Err(UiError::Ai(crate::ai::ai_error::AiError::NoKey));
    }

    Ok(Model { connection, api_key })
}

/// How a question ended.
pub enum Asked {
    /// The whole answer.
    Answer(String),
    /// The user stopped the work.
    Stopped,
    /// The request failed; the reason, for the user.
    Failed(String),
}

/// Asks `model` one `message`. `writing` gets the number of characters
/// written so far, every 400 or so; `stop` ends the request.
pub async fn ask(model: &Model, message: String, stop: &watch::Receiver<bool>, writing: impl Fn(usize)) -> Asked {
    let turns = [Turn {
        speaker: Speaker::Evaluator,
        text: message,
    }];

    // The pieces of the answer only tell the progress; "Stop" drops
    // the receiver, which ends the request (ai/client.rs).
    let (pieces, mut receiver) = mpsc::channel::<String>(256);
    let mut stop_signal = stop.clone();

    let watching = async move {
        let mut characters = 0;
        let mut last_report = 0;

        loop {
            tokio::select! {
                piece = receiver.recv() => match piece {
                    Some(piece) => {
                        characters += piece.chars().count();

                        if characters - last_report >= 400 {
                            last_report = characters;
                            writing(characters);
                        }
                    }
                    None => break,
                },
                _ = stop_signal.wait_for(|stopped| *stopped) => break,
            }
        }
    };

    let asking = async {
        let reply = client::reply(&model.connection, model.api_key.as_ref(), &turns, &pieces).await;
        drop(pieces);
        reply
    };

    let reply = tokio::join!(asking, watching).0;

    if *stop.borrow() {
        return Asked::Stopped;
    }

    match reply.end {
        ReplyEnd::Finished(_) => Asked::Answer(reply.text),
        ReplyEnd::Cancelled => Asked::Stopped,
        ReplyEnd::Failed(error) => Asked::Failed(error.to_string()),
    }
}
