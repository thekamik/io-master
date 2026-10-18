//! Questions to the AI model: the one way the rest of the program asks
//! a model (the chat, the analysis of a standard, later the
//! evaluation). Nothing outside `ai/api/` builds a request or reads an
//! answer of the service (`provider/` is private to this folder).
//!
//! ```text
//! ask()      POST /completions   one question in a task, the reply
//!                                piece by piece, stopped on request,
//!                                and its entry in the journal of the AI
//! counting(), outcome(), single()   help for the long works (the
//!            analysis of a standard): progress, result, one message
//! ```
//!
//! **Every question is in the journal** (`db/ai/journal/`): [`ask`] is
//! the only way out of the program to a model, and it writes the entry
//! itself, also when the reply was stopped or failed. A caller cannot
//! forget it; it learns only whether the entry could be written
//! ([`Asked::journal`]), to tell the evaluator when it could not.
//!
//! A question is asked with the [`Model`] of its role
//! (`roles::model`): the connection, checked, and its key. The rules of
//! the connection protect the key and the conversation
//! (`provider/client.rs`): TLS 1.3, plain HTTP only on the loopback, the
//! key only in a header, no redirect, no proxy.

use std::sync::{Arc, Mutex, PoisonError};

use tokio::sync::{mpsc, watch};

use crate::db::ai::journal as db_journal;
use crate::db::ai::journal::rows::Entry;
use crate::db::error::DbError;
use crate::db::user::database::UserDb;

use super::provider::client;
use super::provider::request;
use super::roles::Model;
use crate::model::ai::Task;

pub use super::provider::ai_error::AiError;
pub use super::provider::client::{Reply, ReplyEnd};
pub use super::provider::response::StopReason;
pub use crate::model::ai::{Speaker, Turn};

/// How many pieces of a reply may wait for the caller.
const PIECE_QUEUE: usize = 256;

/// A question asked, and its entry in the journal of the AI.
#[derive(Debug)]
pub struct Asked {
    pub reply: Reply,
    /// The number of the entry, or why it could not be written (the
    /// reply is kept and shown anyway).
    pub journal: Result<i64, DbError>,
}

/// Asks `model` the next reply of the conversation `turns` in `task`
/// (see [`create`]), and writes the question with its reply into the
/// journal of the AI in the database `db`, on a thread for blocking
/// work.
pub async fn ask(
    db: &Arc<Mutex<UserDb>>,
    model: &Arc<Model>,
    task: Task,
    turns: &[Turn],
    stop: &watch::Receiver<bool>,
    on_piece: impl FnMut(&str) -> bool,
) -> Asked {
    let reply = create(model, turns, stop, on_piece).await;

    let (db, model, turns, written) = (db.clone(), model.clone(), turns.to_vec(), reply.clone());
    let journal = tokio::task::spawn_blocking(move || {
        let db = db.lock().unwrap_or_else(PoisonError::into_inner);

        record(&db, &model, task, &turns, &written)
    })
    .await
    .unwrap_or_else(|_| Err(DbError::Inconsistent("the entry of the journal was not written".to_owned())));

    Asked { reply, journal }
}

/// Asks `model` the next reply of the conversation `turns`; used by
/// [`ask`], which also writes the journal. Every piece
/// of the reply goes to `on_piece` as it arrives; the reply stops when
/// `on_piece` returns `false` (the page is gone) or when `stop` turns
/// `true` ("Stop"). Never fails as a whole: how the reply ended is in
/// [`Reply::end`], next to what was written until then.
pub(in crate::ai) async fn create(
    model: &Model,
    turns: &[Turn],
    stop: &watch::Receiver<bool>,
    mut on_piece: impl FnMut(&str) -> bool,
) -> Reply {
    let (pieces, mut receiver) = mpsc::channel::<String>(PIECE_QUEUE);
    let mut stop = stop.clone();

    // Asks the model; `pieces` is dropped when the reply is done, which
    // ends the passing on below.
    let asking = async move {
        let reply = client::reply(&model.connection, model.api_key.as_ref(), turns, &pieces).await;
        drop(pieces);
        reply
    };

    // Passes the pieces on. On "Stop", or when the caller wants no
    // more, it ends; dropping `receiver` stops the reply
    // (provider/client.rs).
    let passing_on = async move {
        loop {
            tokio::select! {
                piece = receiver.recv() => match piece {
                    Some(piece) => {
                        if !on_piece(&piece) {
                            break;
                        }
                    }
                    None => break,
                },
                _ = stop.wait_for(|stopped| *stopped) => break,
            }
        }
    };

    tokio::join!(asking, passing_on).0
}

/// A handler of the pieces for a long work, which does not show the
/// text: it counts the characters written and calls `report` with the
/// count about every `every` characters, so that the user sees the
/// model working. It never stops the reply.
pub fn counting(every: usize, report: impl Fn(usize)) -> impl FnMut(&str) -> bool {
    let mut characters = 0;
    let mut last_report = 0;

    move |piece| {
        characters += piece.chars().count();

        if characters - last_report >= every {
            last_report = characters;
            report(characters);
        }

        true
    }
}

/// What a question of a long work came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The whole answer.
    Answer(String),
    /// The user stopped the work.
    Stopped,
    /// The request failed; the reason, for the user.
    Failed(String),
}

/// The outcome of `reply`, asked with `stop`.
pub fn outcome(reply: &Reply, stop: &watch::Receiver<bool>) -> Outcome {
    if *stop.borrow() {
        return Outcome::Stopped;
    }

    match &reply.end {
        ReplyEnd::Finished(_) => Outcome::Answer(reply.text.clone()),
        ReplyEnd::Cancelled => Outcome::Stopped,
        ReplyEnd::Failed(error) => Outcome::Failed(error.to_string()),
    }
}

/// One message of the evaluator, as the whole conversation of a
/// question of a long work.
pub fn single(message: String) -> Vec<Turn> {
    vec![Turn { speaker: Speaker::Evaluator, text: message }]
}

/// How a reply ended, as the journal keeps it: complete, cut, refused,
/// stopped or failed, and for a failed reply the reason.
pub fn ending(reply: &Reply) -> (&'static str, Option<String>) {
    match &reply.end {
        ReplyEnd::Finished(StopReason::Complete) => ("complete", None),
        ReplyEnd::Finished(StopReason::Cut) => ("cut", None),
        ReplyEnd::Finished(StopReason::Refused) => ("refused", None),
        ReplyEnd::Cancelled => ("stopped", None),
        ReplyEnd::Failed(error) => ("failed", Some(error.to_string())),
    }
}

/// Writes a question of `task` and its reply into the journal of the
/// AI: the body of the request as it was sent (never a header, so
/// never the key) and the reply as it came, also when it was stopped
/// or failed.
fn record(db: &UserDb, model: &Model, task: Task, turns: &[Turn], reply: &Reply) -> Result<i64, DbError> {
    let request = request::preview(&model.connection, turns);

    write(db, model, task, &request.body.to_string(), reply)
}

/// Writes one entry; used by [`record`] and by `checks.rs`.
pub(super) fn write(db: &UserDb, model: &Model, task: Task, request: &str, reply: &Reply) -> Result<i64, DbError> {
    let (ending, note) = ending(reply);

    db_journal::create::entry(
        db,
        &Entry {
            id: 0,
            time: 0,
            task: task.name().to_owned(),
            role: task.role().map(|role| role.name()).unwrap_or_default().to_owned(),
            connection: model.name.clone(),
            model: reply.model.clone().unwrap_or_else(|| model.connection.model.clone()),
            request: request.to_owned(),
            answer: reply.text.clone(),
            ending: ending.to_owned(),
            note,
            input_tokens: reply.input_tokens,
            output_tokens: reply.output_tokens,
        },
    )
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "completions_tests.rs"]
mod tests;
