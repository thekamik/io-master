//! The test projects in the tool Create evaluation scheme (steps 25 and
//! 27 of `docs/todo.md`): the list of projects, a project with the list
//! of its schemes, the export and import of a project as a file, and
//! the table of contents read by the AI as proposed schemes.
//!
//! The drawing of one scheme is in `scheme_actions.rs`.

use std::path::Path;

use crate::db::user_db::UserDb;
use crate::evaluation::analysis;
use crate::evaluation::checks;
use crate::evaluation::contents;
use crate::evaluation::documents::{self, DocumentFields};
use crate::evaluation::exchange::{self, ExchangeError};
use crate::evaluation::pdf_text;
use crate::evaluation::projects::{self, ProjectData};
use crate::evaluation::scheme::{ProjectFields, SchemeError};
use crate::evaluation::schemes::{self, StoreError};

use super::blocking::blocking;
use super::error::UiError;
use super::model_run::{Asked, active_model, ask};
use super::serializer::project_view::{ProjectDataView, ProjectForm, ProjectView, SchemeForm};
use super::serializer::scheme_view::{AnalysisEvent, PageRange};
use super::serializer::standard_view::{DocumentView, LoadEvent};
use super::state::UiState;
use super::user_data::with_user_db;

impl From<StoreError> for UiError {
    fn from(error: StoreError) -> UiError {
        match error {
            StoreError::Db(error) => UiError::Failed(error.to_string()),
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

/// The project, or the error the page shows when it is gone.
pub fn project_data(db: &UserDb, id: i64) -> Result<ProjectData, UiError> {
    projects::data(db, id)?.ok_or_else(|| UiError::Invalid("the test project does not exist".to_owned()))
}

/// The documents of a project, for the page.
pub fn documents_of(db: &UserDb, data: &ProjectData) -> Result<Vec<DocumentView>, UiError> {
    let mut views = Vec::new();

    for document in &data.project.fields.documents {
        if let Some(document) = documents::get(db, *document)? {
            views.push(DocumentView::from(document));
        }
    }

    Ok(views)
}

/// The view of a project as it is stored now.
fn view(db: &UserDb, id: i64, can_undo: bool) -> Result<ProjectDataView, UiError> {
    let data = project_data(db, id)?;
    let findings = checks::check(&data.tiles, &data.links);
    let documents = documents_of(db, &data)?;

    Ok(ProjectDataView::new(data, &findings, documents, can_undo))
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

    with_user_db(state, move |db| {
        let id = projects::create(db, &fields)?;
        view(db, id, false)
    })
    .await
}

pub async fn open(state: &UiState, id: i64) -> Result<ProjectDataView, UiError> {
    let can_undo = state.can_undo(id);

    with_user_db(state, move |db| view(db, id, can_undo)).await
}

pub async fn update(state: &UiState, id: i64, form: ProjectForm) -> Result<ProjectDataView, UiError> {
    let fields: ProjectFields = form.into();
    let can_undo = state.can_undo(id);

    with_user_db(state, move |db| {
        projects::update(db, id, &fields)?;
        view(db, id, can_undo)
    })
    .await
}

pub async fn set_ready(state: &UiState, id: i64, ready: bool) -> Result<ProjectDataView, UiError> {
    let can_undo = state.can_undo(id);

    with_user_db(state, move |db| {
        projects::set_ready(db, id, ready)?;
        view(db, id, can_undo)
    })
    .await
}

pub async fn delete(state: &UiState, id: i64) -> Result<(), UiError> {
    if !with_user_db(state, move |db| Ok(projects::delete(db, id)?)).await? {
        return Err(UiError::Invalid("the test project does not exist".to_owned()));
    }

    state.forget_undo(id);

    Ok(())
}

/* ---------------------------------------------------------------
 * The schemes of a project
 * ------------------------------------------------------------- */

pub async fn create_scheme(state: &UiState, project: i64, form: SchemeForm) -> Result<ProjectDataView, UiError> {
    let fields = form.into_fields()?;
    let can_undo = state.can_undo(project);

    with_user_db(state, move |db| {
        schemes::create(db, project, &fields)?;
        view(db, project, can_undo)
    })
    .await
}

pub async fn update_scheme(state: &UiState, id: i64, form: SchemeForm) -> Result<ProjectDataView, UiError> {
    let fields = form.into_fields()?;

    let project = with_user_db(state, move |db| {
        let project = schemes::existing(db, id)?.project;
        schemes::update(db, id, &fields)?;
        Ok(project)
    })
    .await?;

    let can_undo = state.can_undo(project);

    with_user_db(state, move |db| view(db, project, can_undo)).await
}

pub async fn accept_schemes(state: &UiState, project: i64, ids: Vec<i64>) -> Result<ProjectDataView, UiError> {
    let can_undo = state.can_undo(project);

    with_user_db(state, move |db| {
        for id in ids {
            if schemes::get(db, id)?.is_some_and(|scheme| scheme.project == project) {
                schemes::accept(db, id)?;
            }
        }

        view(db, project, can_undo)
    })
    .await
}

/// Deletes schemes with their tiles. "Undo" of the project is
/// forgotten: an earlier content names tiles of these schemes.
pub async fn delete_schemes(state: &UiState, project: i64, ids: Vec<i64>) -> Result<ProjectDataView, UiError> {
    with_user_db(state, move |db| {
        for id in ids {
            if schemes::get(db, id)?.is_some_and(|scheme| scheme.project == project) {
                schemes::delete(db, id)?;
            }
        }

        Ok(())
    })
    .await?;

    state.forget_undo(project);

    with_user_db(state, move |db| view(db, project, false)).await
}

/* ---------------------------------------------------------------
 * The table of contents as proposed schemes
 * ------------------------------------------------------------- */

/// Reads the table of contents on the pages `range` with the AI of
/// the active connection, and stores the chapters it finds as proposed
/// schemes of `project`, each with the pages where it was found.
/// Reports to `events`; "Stop" ends it.
pub async fn propose_schemes(
    state: &UiState,
    project: i64,
    range: PageRange,
    events: impl Fn(AnalysisEvent) -> bool + Send + Sync + Clone + 'static,
) -> Result<ProjectDataView, UiError> {
    let (document, part, instructions, model) = with_user_db(state, move |db| {
        let data = project_data(db, project)?;

        if !data.project.fields.documents.contains(&range.document_id) {
            return Err(UiError::Invalid("read only a document of the project".to_owned()));
        }

        let document = documents::get(db, range.document_id)?
            .ok_or_else(|| UiError::Invalid("the document does not exist".to_owned()))?;
        let model = active_model(db)?;

        // The table of contents as one part, however long it is.
        let part = analysis::parts(db, &[(range.document_id, range.first_page, range.last_page)], analysis::PART_RANGE.1)?
            .into_iter()
            .next()
            .ok_or_else(|| UiError::Invalid("the pages of the table of contents hold no text".to_owned()))?;
        let instructions = db
            .setting(contents::INSTRUCTIONS_SETTING)?
            .unwrap_or_else(|| contents::DEFAULT_INSTRUCTIONS.to_owned());

        Ok((document, part, instructions, model))
    })
    .await?;

    let stop = state.start_work();

    events(AnalysisEvent::Part {
        index: 1,
        count: 1,
        document: document.fields.title.clone(),
        first_page: part.first_page,
        last_page: part.last_page,
    });

    let message = contents::prompt(&instructions, &document, &part);
    let writing = {
        let events = events.clone();
        move |characters| {
            events(AnalysisEvent::Writing { characters });
        }
    };

    match ask(&model, message, &stop, writing).await {
        Asked::Answer(answer) => match contents::read(&answer) {
            Ok((chapters, dropped)) => {
                let last = part.last_page;

                let applied = with_user_db(state, move |db| {
                    let starts = contents::locate(db, &document, last, &chapters)?;
                    let spans = contents::spans(&document, &starts);
                    let mut applied = contents::apply(db, project, &chapters, &spans)?;
                    applied.dropped.splice(0..0, dropped);
                    Ok(applied)
                })
                .await?;

                events(AnalysisEvent::Chapters {
                    added: applied.added,
                    known: applied.known,
                    without_pages: applied.without_pages,
                    dropped: applied.dropped,
                });
                events(AnalysisEvent::End { stopped: false });
            }
            Err(reason) => {
                events(AnalysisEvent::Failed { message: reason });
                events(AnalysisEvent::End { stopped: false });
            }
        },
        Asked::Stopped => {
            events(AnalysisEvent::End { stopped: true });
        }
        Asked::Failed(message) => {
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
        exchange::export(db, id, with_pdfs)?.ok_or_else(|| UiError::Invalid("the test project does not exist".to_owned()))
    })
    .await?;

    let path = path.to_owned();

    blocking(move || std::fs::write(&path, file).map_err(|error| UiError::Failed(format!("the file was not written: {error}"))))
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

        if size as usize > exchange::MAX_FILE {
            return Err(UiError::Invalid("the file is larger than 512 MiB".to_owned()));
        }

        let file = std::fs::read(&path).map_err(|error| UiError::Invalid(error.to_string()))?;

        Ok(exchange::read(&file)?)
    })
    .await?;

    let stop = state.start_work();
    let count = exchange.documents.len();
    let mut loaded = Vec::with_capacity(count);
    let mut missing = Vec::new();

    for (index, document) in exchange.documents.iter().enumerate() {
        let sha256 = document.sha256.clone();

        if let Some(found) = with_user_db(state, move |db| Ok(documents::by_sha256(db, &sha256)?)).await? {
            loaded.push(found.id);
            continue;
        }

        let Some(pdf) = document.pdf.clone() else {
            missing.push(format!("{} {} ({})", document.title, document.part, document.file_name).trim().to_owned());
            continue;
        };

        events(LoadEvent::File {
            name: document.file_name.clone(),
            index: index + 1,
            count,
        });

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

        let id = with_user_db(state, move |db| Ok(documents::add(db, &fields, &file_name, &pdf, &pages)?)).await?;
        loaded.push(id);
    }

    events(LoadEvent::End { stopped: false });

    if !missing.is_empty() {
        return Err(UiError::Invalid(format!(
            "load these documents of the standard first, then import again: {}",
            missing.join("; ")
        )));
    }

    with_user_db(state, move |db| {
        let id = exchange::store(db, &exchange, &loaded)?;
        view(db, id, false)
    })
    .await
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "project_actions_tests.rs"]
mod tests;
