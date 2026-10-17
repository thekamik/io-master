//! The drawing of one scheme of a test project in the tool Create
//! evaluation scheme: tiles, links (also to tiles of other schemes of
//! the project), moving tiles between schemes, "Undo", the checks, and
//! the analysis of chapters with AI.
//!
//! The project of the scheme is open for editing in memory
//! (`test_projects/workspace.rs`, through `with_project`). Every action
//! of the user is one change of it ([`change`]): it keeps one version
//! for "Undo" (one per project, since a change may touch two schemes),
//! makes the change in memory after the database has stored it, and the
//! page gets the whole scheme back with its new findings, so that it
//! never has to guess what the program stored.
//!
//! The projects and the list of their schemes are in
//! `project_actions.rs`.

use std::sync::Arc;

use crate::ai::api::completions::{self, Outcome};
use crate::ai::api::roles::{self, Role};
use crate::ai::api::tasks::{self, Task};
use crate::db::user::database::UserDb;
use crate::test_projects::analysis::chapter::{self, Answered};
use crate::test_projects::analysis::parts;
use crate::test_projects::checks;
use crate::test_projects::workspace::OpenProject;
use crate::test_projects::{links, schemes, tiles};

use super::error::UiError;
use super::project_actions::{documents_of, project_of_scheme};
use super::serializer::project_view::ProjectDataView;
use super::serializer::scheme_view::{AnalysisEvent, PageRange, SchemeDataView, TileForm};
use super::state::UiState;
use super::user_data::{with_project, with_user_db};

/// The view of `scheme` of the open project, as it is in memory now.
fn view(db: &UserDb, open: &OpenProject, scheme: i64) -> Result<SchemeDataView, UiError> {
    let data = open.data().clone();
    let findings = checks::check(&data.tiles, &data.links);
    let documents = documents_of(db, &data)?;

    SchemeDataView::new(data, scheme, findings, documents, open.can_undo())
        .ok_or_else(|| UiError::Invalid("the scheme does not exist".to_owned()))
}

/// Makes one change of tiles or links, shown in `scheme`: one version
/// for "Undo", the change, and the scheme as it is afterwards.
async fn change(
    state: &UiState,
    scheme: i64,
    work: impl FnOnce(&UserDb, &mut OpenProject) -> Result<(), UiError> + Send + 'static,
) -> Result<SchemeDataView, UiError> {
    let project = project_of_scheme(state, scheme).await?;

    with_project(state, project, move |db, open| {
        open.change(|open| work(db, open))?;
        view(db, open, scheme)
    })
    .await
}

/// The scheme a tile is in, as stored.
async fn scheme_of_tile(state: &UiState, tile: i64) -> Result<i64, UiError> {
    with_user_db(state, move |db| {
        Ok(crate::db::test_projects::read::tile(db, tile)?
            .ok_or_else(|| UiError::Invalid("the tile does not exist".to_owned()))?
            .scheme)
    })
    .await
}

pub async fn open(state: &UiState, scheme: i64) -> Result<SchemeDataView, UiError> {
    let project = project_of_scheme(state, scheme).await?;

    with_project(state, project, move |db, open| view(db, open, scheme)).await
}

/* ---------------------------------------------------------------
 * The drawing
 * ------------------------------------------------------------- */

pub async fn add_tile(state: &UiState, scheme: i64, form: TileForm) -> Result<SchemeDataView, UiError> {
    let fields = form.into_fields()?;

    change(state, scheme, move |db, open| {
        tiles::create::tile(db, open, scheme, &fields)?;
        Ok(())
    })
    .await
}

pub async fn update_tile(state: &UiState, id: i64, form: TileForm) -> Result<SchemeDataView, UiError> {
    let fields = form.into_fields()?;
    let scheme = scheme_of_tile(state, id).await?;

    change(state, scheme, move |db, open| Ok(tiles::update::tile(db, open, id, &fields)?)).await
}

/// Moves a tile on the canvas; gives nothing back, the page knows
/// where it put it.
pub async fn move_tile(state: &UiState, id: i64, x: f64, y: f64) -> Result<(), UiError> {
    let scheme = scheme_of_tile(state, id).await?;

    change(state, scheme, move |db, open| Ok(tiles::update::position(db, open, id, x, y)?)).await?;

    Ok(())
}

/// Moves tiles of the project to the scheme `to` (step 26); the page
/// gets the scheme `shown` back.
pub async fn move_tiles(state: &UiState, shown: i64, ids: Vec<i64>, to: i64) -> Result<SchemeDataView, UiError> {
    change(state, shown, move |db, open| Ok(tiles::update::to_scheme(db, open, &ids, to)?)).await
}

pub async fn delete_tiles(state: &UiState, scheme: i64, ids: Vec<i64>) -> Result<SchemeDataView, UiError> {
    change(state, scheme, move |db, open| {
        for id in ids {
            if tiles::read::get(open, id).is_some_and(|tile| tile.scheme == scheme) {
                tiles::delete::tile(db, open, id)?;
            }
        }

        Ok(())
    })
    .await
}

pub async fn accept_tiles(state: &UiState, scheme: i64, ids: Vec<i64>) -> Result<SchemeDataView, UiError> {
    change(state, scheme, move |db, open| {
        for id in ids {
            if tiles::read::get(open, id).is_some_and(|tile| tile.scheme == scheme) {
                tiles::update::accept(db, open, id)?;
            }
        }

        Ok(())
    })
    .await
}

/// Links two tiles of the project of `scheme` (one of them may be in
/// another scheme of it).
pub async fn add_link(state: &UiState, scheme: i64, from: i64, to: i64) -> Result<SchemeDataView, UiError> {
    change(state, scheme, move |db, open| {
        if tiles::read::get(open, from).is_none() || tiles::read::get(open, to).is_none() {
            return Err(UiError::Invalid("only tiles of this test project can be linked".to_owned()));
        }

        links::create::link(db, open, from, to)?;
        Ok(())
    })
    .await
}

pub async fn delete_link(state: &UiState, scheme: i64, id: i64) -> Result<SchemeDataView, UiError> {
    change(state, scheme, move |db, open| {
        links::delete::link(db, open, id)?;
        Ok(())
    })
    .await
}

/// Undoes the last change of tiles or links in the project of
/// `scheme` (in any of its schemes); gives `scheme` back.
pub async fn undo(state: &UiState, scheme: i64) -> Result<SchemeDataView, UiError> {
    let project = project_of_scheme(state, scheme).await?;

    with_project(state, project, move |db, open| {
        open.undo(db)?;
        view(db, open, scheme)
    })
    .await
}

/* ---------------------------------------------------------------
 * The analysis with AI
 * ------------------------------------------------------------- */

/// Which instructions: those of the analysis of a chapter, or those
/// of reading a table of contents (`ai/api/tasks.rs`).
fn instruction_task(which: &str) -> Result<Task, UiError> {
    match which {
        "chapter" => Ok(Task::Chapter),
        "contents" => Ok(Task::Contents),
        _ => Err(UiError::Invalid("there are no such instructions".to_owned())),
    }
}

/// The instructions `which` ("chapter" or "contents"): those the user
/// changed, or the default ones.
pub async fn instructions(state: &UiState, which: String) -> Result<String, UiError> {
    let task = instruction_task(&which)?;

    with_user_db(state, move |db| Ok(tasks::instructions(db, task)?.unwrap_or_default())).await
}

/// Stores the instructions `which`; empty text brings the default
/// ones back. Gives the instructions now in force.
pub async fn save_instructions(state: &UiState, which: String, text: String) -> Result<String, UiError> {
    let task = instruction_task(&which)?;

    with_user_db(state, move |db| Ok(tasks::set_instructions(db, task, &text)?.unwrap_or_default())).await
}

/// One scheme to analyse: its number, the pages to read.
struct Job {
    scheme: i64,
    number: String,
    name: String,
    ranges: Vec<(i64, u32, u32)>,
}

/// Analyses one scheme with `ranges` of pages (or, when empty, the
/// pages of the scheme) with the AI (the task `chapter`), part by part
/// (at most `part_size` characters each), and stores what the model
/// proposes as proposed tiles. Reports to `events`; "Stop" ends it.
pub async fn analyse(
    state: &UiState,
    scheme: i64,
    ranges: Vec<PageRange>,
    part_size: usize,
    events: impl Fn(AnalysisEvent) -> bool + Send + Sync + Clone + 'static,
) -> Result<SchemeDataView, UiError> {
    let ranges: Vec<(i64, u32, u32)> =
        ranges.into_iter().map(|range| (range.document_id, range.first_page, range.last_page)).collect();
    let project = project_of_scheme(state, scheme).await?;

    let job = with_project(state, project, move |_, open| {
        let stored = schemes::read::existing(open, scheme)?;
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
            return Err(UiError::Invalid("choose the pages to analyse: the scheme has none".to_owned()));
        }

        Ok(Job { scheme, number: stored.fields.number.clone(), name: stored.fields.name.clone(), ranges })
    })
    .await?;

    run(state, project, vec![job], part_size, events).await?;

    open(state, scheme).await
}

/// Analyses every scheme of `ids` of `project` on its own pages, one
/// after another (a scheme without pages is left out and told).
/// "Undo" takes the whole work back.
pub async fn analyse_schemes(
    state: &UiState,
    project: i64,
    ids: Vec<i64>,
    part_size: usize,
    events: impl Fn(AnalysisEvent) -> bool + Send + Sync + Clone + 'static,
) -> Result<ProjectDataView, UiError> {
    let (jobs, without) = with_project(state, project, move |_, open| {
        let mut jobs = Vec::new();
        let mut without = Vec::new();

        for scheme in open.data().schemes.iter().filter(|scheme| ids.contains(&scheme.id)) {
            let name = format!("{} {}", scheme.fields.number, scheme.fields.name).trim().to_owned();

            match scheme.fields.pages {
                Some(pages) => jobs.push(Job {
                    scheme: scheme.id,
                    number: scheme.fields.number.clone(),
                    name: scheme.fields.name.clone(),
                    ranges: vec![(pages.document_id, pages.first_page, pages.last_page)],
                }),
                None => without.push(name),
            }
        }

        Ok((jobs, without))
    })
    .await?;

    for name in without {
        events(AnalysisEvent::Failed {
            message: format!("{name}: the scheme has no pages; give them in its settings"),
        });
    }

    run(state, project, jobs, part_size, events).await?;

    super::project_actions::open(state, project).await
}

/// The common work of [`analyse`] and [`analyse_schemes`]: every job
/// part by part; one "Undo" for all of it. Each part is the pipeline
/// of `test_projects/analysis/chapter.rs`: the question from the open
/// project, the reply of the model over `ai/api/`, written into the
/// journal of the AI, and the answer applied to the open project.
async fn run(
    state: &UiState,
    project: i64,
    jobs: Vec<Job>,
    part_size: usize,
    events: impl Fn(AnalysisEvent) -> bool + Send + Sync + Clone + 'static,
) -> Result<(), UiError> {
    if jobs.is_empty() {
        events(AnalysisEvent::End { stopped: false });
        return Ok(());
    }

    // Read once: the instructions of the task and the connection of
    // its role; and one version of the project before the work, for
    // "Undo".
    let (model, instructions) = with_project(state, project, move |db, open| {
        let model = roles::model(db, Task::Chapter.role().unwrap_or(Role::TextAnalysis))?;
        let instructions = tasks::instructions(db, Task::Chapter)?.unwrap_or_default();
        open.remember();

        Ok((model, instructions))
    })
    .await?;

    // Shared with the writes of the journal, one per question.
    let model = Arc::new(model);
    let instructions = Arc::new(instructions);
    let stop = state.start_work();

    let count = jobs.len();
    let mut stopped = false;

    'jobs: for (job_index, job) in jobs.into_iter().enumerate() {
        if count > 1
            && !events(AnalysisEvent::Scheme {
                index: job_index + 1,
                count,
                number: job.number.clone(),
                name: job.name.clone(),
            })
        {
            stopped = true;
            break;
        }

        let ranges = job.ranges.clone();
        let parts = with_user_db(state, move |db| Ok(parts::parts(db, &ranges, part_size)?)).await?;
        let parts_count = parts.len();
        let scheme = job.scheme;

        for (index, part) in parts.into_iter().enumerate() {
            if *stop.borrow() {
                stopped = true;
                break 'jobs;
            }

            let (title, turns) = {
                let (part, instructions) = (part.clone(), instructions.clone());

                with_project(state, project, move |db, open| {
                    let title = crate::standards::read::get(db, part.document_id)?
                        .map(|document| document.fields.title)
                        .unwrap_or_default();

                    Ok((title, chapter::question(db, open, scheme, &part, &instructions)?))
                })
                .await?
            };

            if !events(AnalysisEvent::Part {
                index: index + 1,
                count: parts_count,
                document: title,
                first_page: part.first_page,
                last_page: part.last_page,
            }) {
                stopped = true;
                break 'jobs;
            }

            let writing = {
                let events = events.clone();
                move |characters| {
                    events(AnalysisEvent::Writing { characters });
                }
            };

            let reply = completions::create(&model, &turns, &stop, completions::counting(400, writing)).await;
            let outcome = completions::outcome(&reply, &stop);
            record(state, &model, Task::Chapter, turns, reply, &events).await;

            let text = match outcome {
                Outcome::Answer(text) => text,
                Outcome::Stopped => {
                    stopped = true;
                    break 'jobs;
                }
                Outcome::Failed(error) => {
                    events(AnalysisEvent::Failed {
                        message: format!("pages {} to {}: {error}", part.first_page, part.last_page),
                    });
                    continue;
                }
            };

            let (first_page, last_page) = (part.first_page, part.last_page);
            let answer =
                move |db: &UserDb, open: &mut OpenProject| Ok(chapter::answer(db, open, scheme, &part, &text)?);
            let answered = with_project(state, project, answer).await?;

            match answered {
                Answered::Applied(applied) => {
                    events(AnalysisEvent::Applied {
                        added: applied.added,
                        joined: applied.joined,
                        links: applied.links,
                        dropped: applied.dropped,
                    });
                }
                Answered::Unreadable(reason) => {
                    events(AnalysisEvent::Failed { message: format!("pages {first_page} to {last_page}: {reason}") });
                }
            }
        }
    }

    events(AnalysisEvent::End { stopped });

    Ok(())
}

/// Writes a question of a long work into the journal of the AI; a
/// failure is told to the page and does not end the work.
pub(super) async fn record(
    state: &UiState,
    model: &Arc<roles::Model>,
    task: Task,
    turns: Vec<completions::Turn>,
    reply: completions::Reply,
    events: &(impl Fn(AnalysisEvent) -> bool + Send + Sync + Clone + 'static),
) {
    let model = model.clone();
    let written = with_user_db(state, move |db| Ok(completions::record(db, &model, task, &turns, &reply)?)).await;

    if let Err(error) = written {
        events(AnalysisEvent::Failed {
            message: format!("the question was not written into the journal of the AI: {error}"),
        });
    }
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "scheme_actions_tests.rs"]
mod tests;
