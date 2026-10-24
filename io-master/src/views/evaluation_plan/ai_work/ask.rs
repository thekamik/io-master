//! One question of a work of the AI of the view Evaluation plan, with its
//! events: through `ai/api/` (which writes the journal of the AI), the page
//! told how much the model has written and why a question failed. Shared by
//! the proposals of a scheme (`task_scheme/propose.rs`), of the tasks
//! (`task_proposals/propose.rs`) and of the related elements
//! (`task_links/propose.rs`); and the tasks of the AI whose instructions the
//! planner may change.

use std::sync::Arc;

use tauri::ipc::Channel;
use tokio::sync::watch;

use crate::ai::api::completions::{self, Outcome};
use crate::ai::api::roles::Model;
use crate::app::error::AppError;
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
/// Called by: the `propose.rs` of `task_scheme/`, `task_proposals/`, `task_links/`.
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

/// The task of the AI whose instructions are `which`: one of the tasks of
/// the view; refused for any other.
///
/// Called by: `ai_work_instructions_get`, `ai_work_instructions_save` (`evaluation_plan.rs`).
/// Calls: `Task::from_name`.
pub fn task_of(which: &str) -> Result<Task, AppError> {
    let Some(task) = Task::from_name(which) else {
        return Err(AppError::Invalid("there are no such instructions".to_owned()));
    };

    ensure!(
        matches!(task, Task::Scheme | Task::RequirementChoice | Task::RelatedElements),
        AppError::Invalid("there are no such instructions".to_owned())
    );

    Ok(task)
}
