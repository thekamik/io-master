//! The drawing of one scheme of a test project in the tool Create
//! evaluation scheme: tiles, links (also to tiles of other schemes of
//! the project), moving tiles between schemes, "Undo", the checks, and
//! the analysis of chapters with AI.
//!
//! The project of the scheme is open for editing in memory
//! (`business/test_projects/workspace.rs`, through `with_project`). Every action
//! of the user is one change of it ([`change`]): it keeps one version
//! for "Undo" (one per project, since a change may touch two schemes),
//! makes the change in memory after the database has stored it, and the
//! page gets the whole scheme back with its new findings, so that it
//! never has to guess what the program stored.
//!
//! The projects and the list of their schemes are in
//! `commands/projects.rs`.

use crate::ai::api::tasks::{self};
use crate::business::test_projects::analysis::run::{self, Progress};
use crate::business::test_projects::checks;
use crate::business::test_projects::workspace::OpenProject;
use crate::business::test_projects::{links, tiles};
use crate::business::work::Cancel;
use crate::db::user::database::UserDb;
use crate::model::ai::Task;

use super::projects::{documents_of, project_of_scheme};
use crate::tauri_ui::error::UiError;
use crate::tauri_ui::serializer::project_view::ProjectDataView;
use crate::tauri_ui::serializer::scheme_view::{AnalysisEvent, PageRange, SchemeDataView, TileForm};
use crate::tauri_ui::session::{with_project, with_user_db};
use crate::tauri_ui::state::UiState;

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
        tiles::scheme_of(db, tile)?.ok_or_else(|| UiError::Invalid("the tile does not exist".to_owned()))
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
        tiles::create(db, open, scheme, &fields)?;
        Ok(())
    })
    .await
}

pub async fn update_tile(state: &UiState, id: i64, form: TileForm) -> Result<SchemeDataView, UiError> {
    let fields = form.into_fields()?;
    let scheme = scheme_of_tile(state, id).await?;

    change(state, scheme, move |db, open| Ok(tiles::update(db, open, id, &fields)?)).await
}

/// Moves a tile on the canvas; gives nothing back, the page knows
/// where it put it.
pub async fn move_tile(state: &UiState, id: i64, x: f64, y: f64) -> Result<(), UiError> {
    let scheme = scheme_of_tile(state, id).await?;

    change(state, scheme, move |db, open| Ok(tiles::set_position(db, open, id, x, y)?)).await?;

    Ok(())
}

/// Moves tiles of the project to the scheme `to` (step 26); the page
/// gets the scheme `shown` back.
pub async fn move_tiles(state: &UiState, shown: i64, ids: Vec<i64>, to: i64) -> Result<SchemeDataView, UiError> {
    change(state, shown, move |db, open| Ok(tiles::move_to_scheme(db, open, &ids, to)?)).await
}

pub async fn delete_tiles(state: &UiState, scheme: i64, ids: Vec<i64>) -> Result<SchemeDataView, UiError> {
    change(state, scheme, move |db, open| Ok(tiles::delete_in(db, open, scheme, &ids)?)).await
}

pub async fn accept_tiles(state: &UiState, scheme: i64, ids: Vec<i64>) -> Result<SchemeDataView, UiError> {
    change(state, scheme, move |db, open| Ok(tiles::accept_in(db, open, scheme, &ids)?)).await
}

/// Links two tiles of the project of `scheme` (one of them may be in
/// another scheme of it).
pub async fn add_link(state: &UiState, scheme: i64, from: i64, to: i64) -> Result<SchemeDataView, UiError> {
    change(state, scheme, move |db, open| {
        if tiles::get(open, from).is_none() || tiles::get(open, to).is_none() {
            return Err(UiError::Invalid("only tiles of this test project can be linked".to_owned()));
        }

        links::create(db, open, from, to)?;
        Ok(())
    })
    .await
}

pub async fn delete_link(state: &UiState, scheme: i64, id: i64) -> Result<SchemeDataView, UiError> {
    change(state, scheme, move |db, open| {
        links::delete(db, open, id)?;
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
    Task::from_name(which)
        .filter(|task| tasks::default_instructions(*task).is_some())
        .ok_or_else(|| UiError::Invalid("there are no such instructions".to_owned()))
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

/// The stop signal of a long work that starts now.
fn cancel(state: &UiState) -> Cancel {
    Cancel::new(state.start_work())
}

/// Analyses one scheme with `ranges` of pages (or, when empty, the
/// pages of the scheme) with the AI, part by part (at most `part_size`
/// characters each), and stores what the model proposes as proposed
/// tiles (`business/test_projects/analysis/run.rs`). Reports to
/// `events`; "Stop" ends it.
pub async fn analyse(
    state: &UiState,
    scheme: i64,
    ranges: Vec<PageRange>,
    part_size: usize,
    events: impl Fn(AnalysisEvent) -> bool + Send + Sync + Clone + 'static,
) -> Result<SchemeDataView, UiError> {
    let ranges = ranges.into_iter().map(|range| (range.document_id, range.first_page, range.last_page)).collect();
    let project = project_of_scheme(state, scheme).await?;
    let session = state.session()?;
    let progress = move |progress: Progress| events(progress.into());

    run::analyse_scheme(&session, project, scheme, ranges, part_size, cancel(state), progress).await?;

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
    let session = state.session()?;
    let progress = move |progress: Progress| events(progress.into());

    run::analyse_schemes(&session, project, ids, part_size, cancel(state), progress).await?;

    super::projects::open(state, project).await
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "schemes_tests.rs"]
mod tests;
