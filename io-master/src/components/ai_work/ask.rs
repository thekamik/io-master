//! One question of a work of the AI, with its events: through `ai/api/`
//! (which writes the journal of the AI), the page told how much the model
//! has written and why a question failed. Shared by the works of the AI of
//! the views Evaluation plan (the proposals of a scheme, of the tasks, of
//! the related elements) and Evaluation lab (the fragments, the assessment,
//! the outputs).

use std::sync::Arc;

use tauri::ipc::Channel;
use tokio::sync::watch;

use crate::ai::api::completions::{self, Outcome};
use crate::ai::api::roles::Model;
use crate::installation::session::Session;
use crate::model::ai::Task;

use super::view::AiEvent;

/// How often the page learns how much the model has written, in characters.
const WRITING_EVERY: usize = 400;

/// What a question gave.
pub enum Asked {
    /// The whole answer.
    Answer(String),
    /// The question failed; the page was told why.
    Failed,
    /// "Stop", or the page can no longer be reached.
    Stopped,
}

/// Asks `model` the question `text` of `task`, telling the page through
/// `events` how much it has written and why the question failed or was not
/// written into the journal.
///
/// Called by: the works of the AI of the views Evaluation plan and Evaluation lab.
/// Calls: `completions::ask`, `completions::outcome`.
pub async fn ask(
    session: &Session,
    model: &Arc<Model>,
    task: Task,
    text: String,
    stop: &watch::Receiver<bool>,
    events: &Channel<AiEvent>,
) -> Asked {
    // A page that is gone misses the progress; the answer is used all the
    // same, and the next step sees the page is gone and stops the work.
    let writing = |characters| {
        let _page_reached = events.send(AiEvent::Writing { characters }).is_ok();
    };
    let turns = completions::single(text);
    let asked = completions::ask(
        &session.db_handle(),
        model,
        task,
        &turns,
        stop,
        completions::counting(WRITING_EVERY, writing),
    )
    .await;

    if let Err(error) = &asked.journal {
        let message = format!("the question was not written into the journal of the AI: {error}");

        if events.send(AiEvent::Failed { message }).is_err() {
            return Asked::Stopped;
        }
    }

    match completions::outcome(&asked.reply, stop) {
        Outcome::Answer(answer) => Asked::Answer(answer),
        Outcome::Stopped => Asked::Stopped,
        Outcome::Failed(message) => match events.send(AiEvent::Failed { message }) {
            Ok(()) => Asked::Failed,
            Err(_) => Asked::Stopped,
        },
    }
}
