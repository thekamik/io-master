//! The long works of the analysis of a standard with the AI (steps 20
//! and 27 of `docs/todo.md`), moved from the window to the business
//! (step 40): reading a table of contents into proposed schemes, and
//! analysing the pages of schemes into proposed tiles and links.
//!
//! Every question follows one pipeline, and at every moment it is clear
//! where the information is:
//!
//! ```text
//! the open project (memory)  ──question()──►  the text of the question (ai/tasks/)
//!   ──ai/api/completions::ask──►  the model, and the journal of the AI (db/)
//!   ──answer()──►  the proposals checked and written through to the
//!                  project (db/), then made in memory
//! ```
//!
//! The work runs step by step and holds no lock of the session while it
//! waits for the model (`work.rs`). "Stop" ([`Cancel`]) ends it after
//! the step that runs; one "Undo" takes the whole work back.

use std::sync::Arc;

use crate::ai::api::completions::{self, Outcome};
use crate::ai::api::roles::{self, Model};
use crate::ai::api::tasks;
use crate::business::session::Session;
use crate::business::standards;
use crate::business::work::{Cancel, WorkError, on_db, on_project};
use crate::model::ai::{Role, Task, Turn};

use super::chapter::{self, Answered, Applied};
use super::contents::{self, AppliedChapters};
use super::parts;

/// How a long work of the analysis goes, for the evaluator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Progress {
    /// A scheme starts, in an analysis of several (counted from 1).
    Scheme { index: usize, count: usize, number: String, name: String },
    /// A part starts (counted from 1).
    Part { index: usize, count: usize, document: String, first_page: u32, last_page: u32 },
    /// The model is answering: the characters written so far.
    Writing { characters: usize },
    /// What a part of a chapter gave.
    Applied(Applied),
    /// What a table of contents gave.
    Chapters(AppliedChapters),
    /// A step gave nothing; why.
    Failed(String),
    /// The work is over.
    End { stopped: bool },
}

/// One scheme to analyse: its number, the pages to read (document,
/// first page, last page).
#[derive(Debug, Clone)]
struct Job {
    scheme: i64,
    number: String,
    name: String,
    ranges: Vec<(i64, u32, u32)>,
}

/// The model of the role of `task`, and the instructions of the task.
fn model_and_instructions(db: &crate::db::user::database::UserDb, task: Task) -> Result<(Model, String), WorkError> {
    let model = roles::model(db, task.role().unwrap_or(Role::TextAnalysis))?;
    let instructions = tasks::instructions(db, task)?.unwrap_or_default();

    Ok((model, instructions))
}

/// Analyses scheme `scheme` of `project` on `ranges` of pages (or, when
/// empty, on the pages of the scheme), part by part (at most
/// `part_size` characters each).
pub async fn analyse_scheme(
    session: &Arc<Session>,
    project: i64,
    scheme: i64,
    ranges: Vec<(i64, u32, u32)>,
    part_size: usize,
    cancel: Cancel,
    progress: impl Fn(Progress) -> bool + Send + Sync + Clone + 'static,
) -> Result<(), WorkError> {
    let job = on_project(session, project, move |_, open| {
        let stored = super::super::schemes::existing(open, scheme)?;
        let ranges = if ranges.is_empty() {
            stored
                .fields
                .pages
                .map(|pages| vec![(pages.document_id, pages.first_page, pages.last_page)])
                .unwrap_or_default()
        } else {
            ranges
        };

        if ranges.is_empty() {
            return Err(WorkError::Invalid("choose the pages to analyse: the scheme has none".to_owned()));
        }

        Ok(Job { scheme, number: stored.fields.number.clone(), name: stored.fields.name.clone(), ranges })
    })
    .await?;

    run(session, project, vec![job], part_size, cancel, progress).await
}

/// Analyses every scheme of `ids` of `project` on its own pages, one
/// after another; a scheme without pages is left out and told.
pub async fn analyse_schemes(
    session: &Arc<Session>,
    project: i64,
    ids: Vec<i64>,
    part_size: usize,
    cancel: Cancel,
    progress: impl Fn(Progress) -> bool + Send + Sync + Clone + 'static,
) -> Result<(), WorkError> {
    let (jobs, without) = on_project(session, project, move |_, open| {
        let mut jobs = Vec::new();
        let mut without = Vec::new();

        for scheme in open.data().schemes.iter().filter(|scheme| ids.contains(&scheme.id)) {
            match scheme.fields.pages {
                Some(pages) => jobs.push(Job {
                    scheme: scheme.id,
                    number: scheme.fields.number.clone(),
                    name: scheme.fields.name.clone(),
                    ranges: vec![(pages.document_id, pages.first_page, pages.last_page)],
                }),
                None => without.push(format!("{} {}", scheme.fields.number, scheme.fields.name).trim().to_owned()),
            }
        }

        Ok((jobs, without))
    })
    .await?;

    for name in without {
        progress(Progress::Failed(format!("{name}: the scheme has no pages; give them in its settings")));
    }

    run(session, project, jobs, part_size, cancel, progress).await
}

/// The common work of [`analyse_scheme`] and [`analyse_schemes`]: every
/// job part by part; one "Undo" for all of it.
async fn run(
    session: &Arc<Session>,
    project: i64,
    jobs: Vec<Job>,
    part_size: usize,
    cancel: Cancel,
    progress: impl Fn(Progress) -> bool + Send + Sync + Clone + 'static,
) -> Result<(), WorkError> {
    if jobs.is_empty() {
        progress(Progress::End { stopped: false });
        return Ok(());
    }

    // Read once: the connection of the role and the instructions of the
    // task; and one version of the project before the work, for "Undo".
    let (model, instructions) = on_project(session, project, move |db, open| {
        open.remember();
        model_and_instructions(db, Task::Chapter)
    })
    .await?;
    let model = Arc::new(model);
    let instructions = Arc::new(instructions);
    let count = jobs.len();

    let stopped = 'jobs: {
        for (job_index, job) in jobs.into_iter().enumerate() {
            if count > 1
                && !progress(Progress::Scheme {
                    index: job_index + 1,
                    count,
                    number: job.number.clone(),
                    name: job.name.clone(),
                })
            {
                break 'jobs true;
            }

            let ranges = job.ranges.clone();
            let parts = on_db(session, move |db| Ok(parts::parts(db, &ranges, part_size)?)).await?;
            let parts_count = parts.len();
            let scheme = job.scheme;

            for (index, part) in parts.into_iter().enumerate() {
                if cancel.is_cancelled() {
                    break 'jobs true;
                }

                let (title, turns) = {
                    let (part, instructions) = (part.clone(), instructions.clone());

                    on_project(session, project, move |db, open| {
                        let title = standards::documents::get(db, part.document_id)?
                            .map(|document| document.fields.title)
                            .unwrap_or_default();

                        Ok((title, chapter::question(db, open, scheme, &part, &instructions)?))
                    })
                    .await?
                };

                let started = Progress::Part {
                    index: index + 1,
                    count: parts_count,
                    document: title,
                    first_page: part.first_page,
                    last_page: part.last_page,
                };

                if !progress(started) {
                    break 'jobs true;
                }

                let Some(text) = ask(session, &model, Task::Chapter, &turns, &cancel, &progress).await else {
                    break 'jobs true;
                };

                let Some(text) = text else {
                    continue;
                };

                let (first_page, last_page) = (part.first_page, part.last_page);
                let answered =
                    on_project(session, project, move |db, open| Ok(chapter::answer(db, open, scheme, &part, &text)?))
                        .await?;

                match answered {
                    Answered::Applied(applied) => {
                        progress(Progress::Applied(applied));
                    }
                    Answered::Unreadable(reason) => {
                        progress(Progress::Failed(format!("pages {first_page} to {last_page}: {reason}")));
                    }
                }
            }
        }

        false
    };

    progress(Progress::End { stopped });

    Ok(())
}

/// Asks `model` one question of a long work, reporting the writing and a
/// journal that could not be written. `None`: the work was stopped;
/// `Some(None)`: the question failed (told); `Some(Some(text))`: the
/// answer.
async fn ask(
    session: &Arc<Session>,
    model: &Arc<Model>,
    task: Task,
    turns: &[Turn],
    cancel: &Cancel,
    progress: &(impl Fn(Progress) -> bool + Send + Sync + Clone + 'static),
) -> Option<Option<String>> {
    let writing = {
        let progress = progress.clone();
        move |characters| {
            progress(Progress::Writing { characters });
        }
    };

    let asked = completions::ask(
        &session.db_handle(),
        model,
        task,
        turns,
        cancel.signal(),
        completions::counting(400, writing),
    )
    .await;

    if let Err(error) = &asked.journal {
        progress(Progress::Failed(format!("the question was not written into the journal of the AI: {error}")));
    }

    match completions::outcome(&asked.reply, cancel.signal()) {
        Outcome::Answer(text) => Some(Some(text)),
        Outcome::Stopped => None,
        Outcome::Failed(error) => {
            progress(Progress::Failed(error));
            Some(None)
        }
    }
}

/// Reads the table of contents on the pages `first_page` to `last_page`
/// of `document` with the AI (the task `contents`), and stores the
/// chapters it finds as proposed schemes of `project`, each with the
/// pages where it was found.
pub async fn propose_schemes(
    session: &Arc<Session>,
    project: i64,
    document: i64,
    first_page: u32,
    last_page: u32,
    cancel: Cancel,
    progress: impl Fn(Progress) -> bool + Send + Sync + Clone + 'static,
) -> Result<(), WorkError> {
    let (question, model) = on_project(session, project, move |db, open| {
        let (model, instructions) = model_and_instructions(db, Task::Contents)?;
        let question = contents::question(db, open, document, first_page, last_page, &instructions)?;

        Ok((question, model))
    })
    .await?;

    progress(Progress::Part {
        index: 1,
        count: 1,
        document: question.document.fields.title.clone(),
        first_page: question.part.first_page,
        last_page: question.part.last_page,
    });

    let model = Arc::new(model);

    let stopped = match ask(session, &model, Task::Contents, &question.turns, &cancel, &progress).await {
        None => true,
        Some(None) => false,
        Some(Some(text)) => {
            let answered =
                on_project(session, project, move |db, open| Ok(contents::answer(db, open, &question, &text)?)).await?;

            match answered {
                contents::Answered::Applied(applied) => {
                    progress(Progress::Chapters(applied));
                }
                contents::Answered::Unreadable(reason) => {
                    progress(Progress::Failed(reason));
                }
            }

            false
        }
    };

    progress(Progress::End { stopped });

    Ok(())
}
