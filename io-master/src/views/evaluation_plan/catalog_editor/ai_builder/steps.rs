//! The steps both works of the AI share: the model and the instructions
//! of a task, the parts of the pages, one question with its events, and
//! the quotes of a proposal checked on their pages.

use std::sync::Arc;

use tauri::ipc::Channel;
use tokio::sync::watch;

use crate::ai::api::completions::{self, Outcome};
use crate::ai::api::roles::{self, Model};
use crate::ai::api::tasks;
use crate::app::error::AppError;
use crate::db::standards as db_standards;
use crate::db::user::database::UserDb;
use crate::installation::session::Session;
use crate::model::ai::{Role, Task, Turn};
use crate::model::evaluation_support::QuoteDraft;
use crate::model::standards::{Document, Part};

use super::super::checked_quote;
use super::parts;
use super::view::BuildEvent;

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

/// The model of the role of `task`, and the instructions of the task.
pub fn model_and_instructions(db: &UserDb, task: Task) -> Result<(Model, String), AppError> {
    let role = match task.role() {
        Some(role) => role,
        None => Role::TextAnalysis,
    };

    let model = roles::model(db, role)?;
    let instructions = tasks::instructions(db, task)?.unwrap_or_default();

    Ok((model, instructions))
}

/// The parts of the pages `ranges` (document, first page, last page), each
/// with its document.
pub fn parts_with_documents(
    db: &UserDb,
    ranges: &[(i64, u32, u32)],
    part_size: usize,
) -> Result<Vec<(Part, Document)>, AppError> {
    let mut found = Vec::new();

    for part in parts::parts(db, ranges, part_size)? {
        let Some(document) = db_standards::read::document(db, part.document_id)? else {
            return Err(AppError::Invalid("the document of the standard does not exist".to_owned()));
        };

        found.push((part, document));
    }

    Ok(found)
}

/// The event that part `index` (from 0) of `count` starts.
pub fn part_event(index: usize, count: usize, part: &Part, document: &Document) -> BuildEvent {
    BuildEvent::Part {
        index: index + 1,
        count,
        document: document.fields.title.clone(),
        first_page: part.first_page,
        last_page: part.last_page,
    }
}

/// Asks `model` one question, telling the page how much it has written
/// and why a question failed or was not written into the journal.
pub async fn ask(
    session: &Session,
    model: &Arc<Model>,
    task: Task,
    turns: &[Turn],
    stop: &watch::Receiver<bool>,
    events: &Channel<BuildEvent>,
) -> Asked {
    // A page that is gone misses the progress, and nothing can be done
    // about it here: the answer is stored all the same, and the next step
    // sees the page is gone and stops the work.
    let writing = |characters| {
        let _page_reached = events.send(BuildEvent::Writing { characters }).is_ok();
    };

    let asked =
        completions::ask(&session.db_handle(), model, task, turns, stop, completions::counting(WRITING_EVERY, writing))
            .await;

    if let Err(error) = &asked.journal {
        let message = format!("the question was not written into the journal of the AI: {error}");

        if events.send(BuildEvent::Failed { message }).is_err() {
            return Asked::Stopped;
        }
    }

    match completions::outcome(&asked.reply, stop) {
        Outcome::Answer(text) => Asked::Answer(text),
        Outcome::Stopped => Asked::Stopped,
        Outcome::Failed(message) => match events.send(BuildEvent::Failed { message }) {
            Ok(()) => Asked::Failed,
            Err(_) => Asked::Stopped,
        },
    }
}

/// The quotes of a proposal of `part`, looked for on their pages; a quote
/// that cannot be checked (a page the document does not have, an empty
/// text) is left out, as the proposal is shown without it.
pub fn checked_quotes(db: &UserDb, part: &Part, quotes: &[(u32, String)]) -> Vec<QuoteDraft> {
    let mut checked = Vec::with_capacity(quotes.len());

    for (page, text) in quotes {
        if let Ok(quote) = checked_quote::checked(db, part.document_id, *page, text) {
            checked.push(quote);
        }
    }

    checked
}
