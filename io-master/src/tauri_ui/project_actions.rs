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
use std::sync::Arc;

use crate::ai::api::completions::{self, Outcome};
use crate::ai::api::roles::{self};
use crate::ai::api::tasks::{self};
use crate::business::standards;
use crate::business::standards::pdf_text;
use crate::business::test_projects::analysis::contents::{self, Answered};
use crate::business::test_projects::checks;
use crate::business::test_projects::error::StoreError;
use crate::business::test_projects::exchange::format::{self, ExchangeError};
use crate::business::test_projects::exchange::{export, import};
use crate::business::test_projects::workspace::OpenProject;
use crate::business::test_projects::{projects, schemes};
use crate::db::user::database::UserDb;
use crate::model::ai::Role;
use crate::model::ai::Task;
use crate::model::standards::DocumentFields;
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
        if let Some(document) = standards::read::get(db, *document)? {
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
    let projects = with_user_db(state, |db| Ok(projects::read::list(db)?)).await?;

    Ok(projects.into_iter().map(ProjectView::from).collect())
}

pub async fn create(state: &UiState, form: ProjectForm) -> Result<ProjectDataView, UiError> {
    let fields: ProjectFields = form.into();
    let id = with_user_db(state, move |db| Ok(projects::create::project(db, &fields)?)).await?;

    open(state, id).await
}

/// Opens a project for editing (from the database, the first time).
pub async fn open(state: &UiState, id: i64) -> Result<ProjectDataView, UiError> {
    with_project(state, id, |db, open| view(db, open)).await
}

pub async fn update(state: &UiState, id: i64, form: ProjectForm) -> Result<ProjectDataView, UiError> {
    let fields: ProjectFields = form.into();

    with_project(state, id, move |db, open| {
        projects::update::fields(db, open, &fields)?;
        view(db, open)
    })
    .await
}

pub async fn set_ready(state: &UiState, id: i64, ready: bool) -> Result<ProjectDataView, UiError> {
    with_project(state, id, move |db, open| {
        projects::update::ready(db, open, ready)?;
        view(db, open)
    })
    .await
}

pub async fn delete(state: &UiState, id: i64) -> Result<(), UiError> {
    if !with_workspace(state, move |db, workspace| Ok(projects::delete::project(db, workspace, id)?)).await? {
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
        schemes::create::scheme(db, open, &fields)?;
        view(db, open)
    })
    .await
}

pub async fn update_scheme(state: &UiState, id: i64, form: SchemeForm) -> Result<ProjectDataView, UiError> {
    let fields = form.into_fields()?;
    let project = project_of_scheme(state, id).await?;

    with_project(state, project, move |db, open| {
        schemes::update::scheme(db, open, id, &fields)?;
        view(db, open)
    })
    .await
}

pub async fn accept_schemes(state: &UiState, project: i64, ids: Vec<i64>) -> Result<ProjectDataView, UiError> {
    with_project(state, project, move |db, open| {
        for id in ids {
            if schemes::read::get(open, id).is_some() {
                schemes::update::accept(db, open, id)?;
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
            schemes::delete::scheme(db, open, id)?;
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
/// task `contents`, in the role of its connection), and stores the
/// chapters it finds as proposed schemes of `project`, each with the
/// pages where it was found. Reports to `events`; "Stop" ends it.
pub async fn propose_schemes(
    state: &UiState,
    project: i64,
    range: PageRange,
    events: impl Fn(AnalysisEvent) -> bool + Send + Sync + Clone + 'static,
) -> Result<ProjectDataView, UiError> {
    let (question, model) = with_project(state, project, move |db, open| {
        let instructions = tasks::instructions(db, Task::Contents)?.unwrap_or_default();
        let question =
            contents::question(db, open, range.document_id, range.first_page, range.last_page, &instructions)?;
        let model = roles::model(db, Task::Contents.role().unwrap_or(Role::TextAnalysis))?;

        Ok((question, model))
    })
    .await?;

    let stop = state.start_work();

    events(AnalysisEvent::Part {
        index: 1,
        count: 1,
        document: question.document.fields.title.clone(),
        first_page: question.part.first_page,
        last_page: question.part.last_page,
    });

    let writing = {
        let events = events.clone();
        move |characters| {
            events(AnalysisEvent::Writing { characters });
        }
    };

    let model = Arc::new(model);
    let reply = completions::create(&model, &question.turns, &stop, completions::counting(400, writing)).await;
    let outcome = completions::outcome(&reply, &stop);
    super::scheme_actions::record(state, &model, Task::Contents, question.turns.clone(), reply, &events).await;

    match outcome {
        Outcome::Answer(text) => {
            let answered =
                with_project(state, project, move |db, open| Ok(contents::answer(db, open, &question, &text)?)).await?;

            match answered {
                Answered::Applied(applied) => {
                    events(AnalysisEvent::Chapters {
                        added: applied.added,
                        known: applied.known,
                        without_pages: applied.without_pages,
                        dropped: applied.dropped,
                    });
                }
                Answered::Unreadable(reason) => {
                    events(AnalysisEvent::Failed { message: reason });
                }
            }

            events(AnalysisEvent::End { stopped: false });
        }
        Outcome::Stopped => {
            events(AnalysisEvent::End { stopped: true });
        }
        Outcome::Failed(message) => {
            events(AnalysisEvent::Failed { message });
            events(AnalysisEvent::End { stopped: false });
        }
    }

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

/// Imports the file of a project at `path`. Its documents are found
/// among the loaded ones by their SHA-256, or loaded from the PDFs the
/// file carries (reported to `events` like any loading); a document
/// that is neither is named in the error, to be loaded first.
pub async fn import(
    state: &UiState,
    path: &Path,
    events: impl Fn(LoadEvent) -> bool + Send + Sync + Clone + 'static,
) -> Result<ProjectDataView, UiError> {
    state.session()?;

    let path = path.to_owned();
    let exchange = blocking(move || {
        let size = std::fs::metadata(&path).map_err(|error| UiError::Invalid(error.to_string()))?.len();

        if size as usize > format::MAX_FILE {
            return Err(UiError::Invalid("the file is larger than 512 MiB".to_owned()));
        }

        let file = std::fs::read(&path).map_err(|error| UiError::Invalid(error.to_string()))?;

        Ok(import::read(&file)?)
    })
    .await?;

    let stop = state.start_work();
    let count = exchange.documents.len();
    let mut loaded = Vec::with_capacity(count);
    let mut missing = Vec::new();

    for (index, document) in exchange.documents.iter().enumerate() {
        let sha256 = document.sha256.clone();

        if let Some(found) = with_user_db(state, move |db| Ok(standards::read::by_sha256(db, &sha256)?)).await? {
            loaded.push(found.id);
            continue;
        }

        let Some(pdf) = document.pdf.clone() else {
            missing.push(format!("{} {} ({})", document.title, document.part, document.file_name).trim().to_owned());
            continue;
        };

        events(LoadEvent::File { name: document.file_name.clone(), index: index + 1, count });

        let reading = {
            let events = events.clone();
            let stop = stop.clone();

            blocking(move || {
                let info = pdf_text::info(&pdf).map_err(|error| UiError::Invalid(error.to_string()))?;
                let pages = pdf_text::pages(&pdf, info.pages, |done, total| {
                    !*stop.borrow() && events(LoadEvent::Pages { done, total })
                })
                .map_err(|error| UiError::Invalid(error.to_string()))?;

                Ok((pdf, pages))
            })
            .await
        };

        let (pdf, pages) = reading?;
        let fields = DocumentFields {
            title: document.title.clone(),
            part: document.part.clone(),
            version: document.version.clone(),
        };
        let file_name = document.file_name.clone();

        let id =
            with_user_db(state, move |db| Ok(standards::create::add(db, &fields, &file_name, &pdf, &pages)?)).await?;
        loaded.push(id);
    }

    events(LoadEvent::End { stopped: false });

    if !missing.is_empty() {
        return Err(UiError::Invalid(format!(
            "load these documents of the standard first, then import again: {}",
            missing.join("; ")
        )));
    }

    let id = with_user_db(state, move |db| Ok(import::store(db, &exchange, &loaded)?)).await?;

    open(state, id).await
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "project_actions_tests.rs"]
mod tests;
