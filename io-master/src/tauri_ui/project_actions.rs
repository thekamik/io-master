//! The test projects in the tool Create evaluation scheme (steps 25 and
//! 27 of `docs/todo.md`): the list of projects, a project with the list
//! of its schemes, the export and import of a project as a file, and
//! the table of contents read by the AI as proposed schemes.
//!
//! A project the page shows is open for editing in memory
//! (`business/test_projects/workspace.rs`, through `with_project`): every change
//! is made on it and written through to the database at once. The page
//! gets the project back as it is in memory after every change.
//!
//! The drawing of one scheme is in `scheme_actions.rs`.

use std::path::Path;

use crate::business::standards;
use crate::business::standards::load::LoadProgress;
use crate::business::test_projects::analysis::run::{self, Progress};
use crate::business::test_projects::checks;
use crate::business::test_projects::error::StoreError;
use crate::business::test_projects::exchange::format::ExchangeError;
use crate::business::test_projects::exchange::{export, import};
use crate::business::test_projects::workspace::OpenProject;
use crate::business::test_projects::{projects, schemes};
use crate::business::work::Cancel;
use crate::db::user::database::UserDb;
use crate::model::test_projects::rules::SchemeError;
use crate::model::test_projects::{ProjectData, ProjectFields};

use super::blocking::blocking;
use super::error::UiError;
use super::serializer::project_view::{ProjectDataView, ProjectForm, ProjectView, SchemeForm};
use super::serializer::scheme_view::{AnalysisEvent, PageRange};
use super::serializer::standard_view::{DocumentView, LoadEvent};
use super::state::UiState;
use super::user_data::{with_project, with_user_db, with_workspace};

impl From<StoreError> for UiError {
    fn from(error: StoreError) -> UiError {
        match error {
            StoreError::Db(error) => UiError::Failed(error.to_string()),
            StoreError::Scheme(SchemeError::NotFound("project")) => {
                UiError::Invalid("the test project does not exist".to_owned())
            }
            StoreError::Scheme(error) => UiError::Invalid(error.to_string()),
        }
    }
}

impl From<SchemeError> for UiError {
    fn from(error: SchemeError) -> UiError {
        UiError::Invalid(error.to_string())
    }
}

impl From<ExchangeError> for UiError {
    fn from(error: ExchangeError) -> UiError {
        UiError::Invalid(error.to_string())
    }
}

/// The documents of a project, for the page.
pub fn documents_of(db: &UserDb, data: &ProjectData) -> Result<Vec<DocumentView>, UiError> {
    let mut views = Vec::new();

    for document in &data.project.fields.documents {
        if let Some(document) = standards::documents::get(db, *document)? {
            views.push(DocumentView::from(document));
        }
    }

    Ok(views)
}

/// The view of an open project, as it is in memory now.
pub fn view(db: &UserDb, open: &OpenProject) -> Result<ProjectDataView, UiError> {
    let data = open.data().clone();
    let findings = checks::check(&data.tiles, &data.links);
    let documents = documents_of(db, &data)?;

    Ok(ProjectDataView::new(data, &findings, documents, open.can_undo()))
}

/* ---------------------------------------------------------------
 * Projects
 * ------------------------------------------------------------- */

pub async fn list(state: &UiState) -> Result<Vec<ProjectView>, UiError> {
    let projects = with_user_db(state, |db| Ok(projects::list(db)?)).await?;

    Ok(projects.into_iter().map(ProjectView::from).collect())
}

pub async fn create(state: &UiState, form: ProjectForm) -> Result<ProjectDataView, UiError> {
    let fields: ProjectFields = form.into();
    let id = with_user_db(state, move |db| Ok(projects::create(db, &fields)?)).await?;

    open(state, id).await
}

/// Opens a project for editing (from the database, the first time).
pub async fn open(state: &UiState, id: i64) -> Result<ProjectDataView, UiError> {
    with_project(state, id, |db, open| view(db, open)).await
}

pub async fn update(state: &UiState, id: i64, form: ProjectForm) -> Result<ProjectDataView, UiError> {
    let fields: ProjectFields = form.into();

    with_project(state, id, move |db, open| {
        projects::update(db, open, &fields)?;
        view(db, open)
    })
    .await
}

pub async fn set_ready(state: &UiState, id: i64, ready: bool) -> Result<ProjectDataView, UiError> {
    with_project(state, id, move |db, open| {
        projects::set_ready(db, open, ready)?;
        view(db, open)
    })
    .await
}

pub async fn delete(state: &UiState, id: i64) -> Result<(), UiError> {
    if !with_workspace(state, move |db, workspace| Ok(projects::delete(db, workspace, id)?)).await? {
        return Err(UiError::Invalid("the test project does not exist".to_owned()));
    }

    Ok(())
}

/* ---------------------------------------------------------------
 * The schemes of a project
 * ------------------------------------------------------------- */

pub async fn create_scheme(state: &UiState, project: i64, form: SchemeForm) -> Result<ProjectDataView, UiError> {
    let fields = form.into_fields()?;

    with_project(state, project, move |db, open| {
        schemes::create(db, open, &fields)?;
        view(db, open)
    })
    .await
}

pub async fn update_scheme(state: &UiState, id: i64, form: SchemeForm) -> Result<ProjectDataView, UiError> {
    let fields = form.into_fields()?;
    let project = project_of_scheme(state, id).await?;

    with_project(state, project, move |db, open| {
        schemes::update(db, open, id, &fields)?;
        view(db, open)
    })
    .await
}

pub async fn accept_schemes(state: &UiState, project: i64, ids: Vec<i64>) -> Result<ProjectDataView, UiError> {
    with_project(state, project, move |db, open| {
        for id in ids {
            if schemes::get(open, id).is_some() {
                schemes::accept(db, open, id)?;
            }
        }

        view(db, open)
    })
    .await
}

/// Deletes schemes with their tiles. "Undo" of the project is
/// forgotten: an earlier content names tiles of these schemes.
pub async fn delete_schemes(state: &UiState, project: i64, ids: Vec<i64>) -> Result<ProjectDataView, UiError> {
    with_project(state, project, move |db, open| {
        for id in ids {
            schemes::delete(db, open, id)?;
        }

        view(db, open)
    })
    .await
}

/// The project a scheme belongs to, as stored.
pub async fn project_of_scheme(state: &UiState, scheme: i64) -> Result<i64, UiError> {
    with_user_db(state, move |db| {
        Ok(crate::db::test_projects::read::scheme(db, scheme)?
            .ok_or_else(|| UiError::Invalid("the scheme does not exist".to_owned()))?
            .project)
    })
    .await
}

/* ---------------------------------------------------------------
 * The table of contents as proposed schemes
 * ------------------------------------------------------------- */

/// Reads the table of contents on the pages `range` with the AI (the
/// task `contents`), and stores the chapters it finds as proposed
/// schemes of `project` (`business/test_projects/analysis/run.rs`).
/// Reports to `events`; "Stop" ends it.
pub async fn propose_schemes(
    state: &UiState,
    project: i64,
    range: PageRange,
    events: impl Fn(AnalysisEvent) -> bool + Send + Sync + Clone + 'static,
) -> Result<ProjectDataView, UiError> {
    let session = state.session()?;
    let cancel = Cancel::new(state.start_work());
    let progress = move |progress: Progress| events(progress.into());

    run::propose_schemes(&session, project, range.document_id, range.first_page, range.last_page, cancel, progress)
        .await?;

    open(state, project).await
}

/* ---------------------------------------------------------------
 * Export and import
 * ------------------------------------------------------------- */

/// The file of a project (with the PDFs when `with_pdfs`), written to
/// `path`.
pub async fn export(state: &UiState, id: i64, with_pdfs: bool, path: &Path) -> Result<(), UiError> {
    let file = with_user_db(state, move |db| {
        export::export(db, id, with_pdfs)?.ok_or_else(|| UiError::Invalid("the test project does not exist".to_owned()))
    })
    .await?;

    let path = path.to_owned();

    blocking(move || {
        std::fs::write(&path, file).map_err(|error| UiError::Failed(format!("the file was not written: {error}")))
    })
    .await
}

/// Imports the file of a project at `path`
/// (`business/test_projects/exchange/import.rs`); its documents are
/// found among the loaded ones or loaded from the file, reported to
/// `events` like any loading.
pub async fn import(
    state: &UiState,
    path: &Path,
    events: impl Fn(LoadEvent) -> bool + Send + Sync + Clone + 'static,
) -> Result<ProjectDataView, UiError> {
    let session = state.session()?;
    let cancel = Cancel::new(state.start_work());
    let progress = move |progress: LoadProgress| events(progress.into());

    let id = import::from_file(&session, path.to_owned(), cancel, progress).await?;

    open(state, id).await
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "project_actions_tests.rs"]
mod tests;
