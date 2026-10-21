//! The long works that make a catalog of OSCAL from the documents of a
//! standard (PDF) with the AI (step 41.3): the groups from a table of
//! contents (`propose_groups`), the controls of a group from its pages
//! (`propose_controls`). What the AI proposes is never written into the
//! catalog: it waits as a proposal of the evaluation support
//! (`proposals.rs`), with its quotes looked for on their pages, until
//! the evaluator accepts or rejects it.
//!
//! Every question follows one pipeline:
//!
//! ```text
//! the catalog and the pages (db/)  ──prompt (ai/tasks/catalog_*.rs)──►  the question
//!   ──ai/api/completions::ask──►  the model, and the journal of the AI
//!   ──read (ai/tasks/catalog_*.rs)──►  groups or controls of OSCAL
//!   ──quotes checked, proposals stored (db/evaluation_support/)
//! ```
//!
//! The work runs step by step and holds no lock of the session while it
//! waits for the model (`business/work.rs`); "Stop" ends it after the
//! step that runs.

use std::sync::Arc;

use crate::ai::api::completions::{self, Outcome};
use crate::ai::api::roles::{self, Model};
use crate::ai::api::tasks;
use crate::ai::tasks::{catalog_outline, catalog_section};
use crate::business::standards;
use crate::business::standards::parts;
use crate::business::work::{Cancel, WorkError, on_db};
use crate::db::evaluation_support as db_evaluation_support;
use crate::db::evaluation_support::create::NewProposal;
use crate::db::user::database::UserDb;
use crate::installation::session::Session;
use crate::model::ai::{Role, Task, Turn};
use crate::model::evaluation_support::{ProposalKind, QuoteDraft};
use crate::model::oscal::catalog::Catalog;
use crate::model::standards::Part;

use super::catalogs;
use super::documents::OscalError;
use super::quotes;

/// How a long work of building a catalog goes, for the evaluator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Progress {
    /// A part starts (counted from 1).
    Part { index: usize, count: usize, document: String, first_page: u32, last_page: u32 },
    /// The model is answering: the characters written so far.
    Writing { characters: usize },
    /// What a part gave: new proposals, and what was dropped and why.
    Proposed { added: usize, dropped: Vec<String> },
    /// A step gave nothing; why.
    Failed(String),
    /// The work is over.
    End { stopped: bool },
}

impl From<OscalError> for WorkError {
    fn from(error: OscalError) -> WorkError {
        match error {
            OscalError::Db(error) => WorkError::Db(error),
            other => WorkError::Invalid(other.to_string()),
        }
    }
}

/// The model of the role of `task`, and the instructions of the task.
fn model_and_instructions(db: &UserDb, task: Task) -> Result<(Model, String), WorkError> {
    let model = roles::model(db, task.role().unwrap_or(Role::TextAnalysis))?;
    let instructions = tasks::instructions(db, task)?.unwrap_or_default();

    Ok((model, instructions))
}

/// The groups of a catalog (id, title), as the model is told them.
fn known_groups(catalog: &Catalog) -> Vec<(String, String)> {
    fn walk(groups: &[crate::model::oscal::catalog::Group], out: &mut Vec<(String, String)>) {
        for group in groups {
            if let Some(id) = &group.id {
                out.push((id.clone(), group.title.clone()));
            }

            walk(&group.groups, out);
        }
    }

    let mut out = Vec::new();
    walk(&catalog.groups, &mut out);

    out
}

/// The controls of a catalog (id, title), as the model is told them.
fn known_controls(catalog: &Catalog) -> Vec<(String, String)> {
    catalog.all_controls().into_iter().map(|(_, control)| (control.id.clone(), control.title.clone())).collect()
}

/// The quotes of a proposal, looked for on their pages; a quote that
/// cannot be checked (a page the document does not have) is left out.
fn checked_quotes(db: &UserDb, part: &Part, quotes_: &[(u32, String)]) -> Vec<QuoteDraft> {
    quotes_.iter().filter_map(|(page, text)| quotes::checked(db, part.document_id, *page, text).ok()).collect()
}

/// Asks `model` one question of a long work, reporting the writing and
/// a journal that could not be written. `None`: the work was stopped;
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

/// The parts of the pages `ranges` (document, first page, last page),
/// with the title of their document.
async fn parts_of(
    session: &Arc<Session>,
    ranges: Vec<(i64, u32, u32)>,
    part_size: usize,
) -> Result<Vec<(Part, crate::model::standards::Document)>, WorkError> {
    on_db(session, move |db| {
        let mut out = Vec::new();

        for part in parts::parts(db, &ranges, part_size)? {
            let document = standards::documents::get(db, part.document_id)?
                .ok_or(WorkError::Invalid("the document of the standard does not exist".to_owned()))?;
            out.push((part, document));
        }

        Ok(out)
    })
    .await
}

/// Reads a table of contents (the pages `first` to `last` of document
/// `source` of a standard) and proposes the groups of catalog `uuid`.
pub async fn propose_groups(
    session: &Arc<Session>,
    uuid: String,
    source: i64,
    first: u32,
    last: u32,
    cancel: Cancel,
    progress: impl Fn(Progress) -> bool + Send + Sync + Clone + 'static,
) -> Result<(), WorkError> {
    let (model, instructions) = {
        let uuid = uuid.clone();
        on_db(session, move |db| {
            catalogs::get(db, &uuid)?;
            model_and_instructions(db, Task::CatalogOutline)
        })
        .await?
    };
    let model = Arc::new(model);
    let parts = parts_of(session, vec![(source, first, last)], 0).await?;
    let count = parts.len();

    let stopped = 'parts: {
        for (index, (part, document)) in parts.into_iter().enumerate() {
            if cancel.is_cancelled() {
                break 'parts true;
            }

            let known = {
                let uuid = uuid.clone();
                on_db(session, move |db| Ok(known_groups(&catalogs::get(db, &uuid)?))).await?
            };
            let turns = completions::single(catalog_outline::prompt(&instructions, &document, &part, &known));

            let started = Progress::Part {
                index: index + 1,
                count,
                document: document.fields.title.clone(),
                first_page: part.first_page,
                last_page: part.last_page,
            };

            if !progress(started) {
                break 'parts true;
            }

            let Some(answer) = ask(session, &model, Task::CatalogOutline, &turns, &cancel, &progress).await else {
                break 'parts true;
            };

            let Some(answer) = answer else {
                continue;
            };

            match catalog_outline::read(&answer, &part) {
                Ok(outline) => {
                    let (uuid, name) = (uuid.clone(), model.name.clone());
                    let dropped = outline.dropped.clone();
                    let added = on_db(session, move |db| {
                        for proposed in &outline.groups {
                            let quotes_ = checked_quotes(db, &part, proposed.quote.as_slice());
                            let content = serde_json::to_value(&proposed.group)
                                .map_err(|error| WorkError::Invalid(error.to_string()))?;

                            db_evaluation_support::create::proposal(
                                db,
                                &NewProposal {
                                    document_uuid: &uuid,
                                    kind: ProposalKind::Group,
                                    parent_id: proposed.parent.as_deref(),
                                    content: &content,
                                    quotes: &quotes_,
                                    note: "",
                                    model: &name,
                                },
                            )?;
                        }

                        Ok(outline.groups.len())
                    })
                    .await?;

                    progress(Progress::Proposed { added, dropped });
                }
                Err(reason) => {
                    progress(Progress::Failed(reason));
                }
            }
        }

        false
    };

    progress(Progress::End { stopped });

    Ok(())
}

/// Reads the pages `ranges` (document, first page, last page) of a
/// group of a standard, part by part (at most `part_size` characters
/// each), and proposes the controls of group `group` (`None`: the top)
/// of catalog `uuid`.
pub async fn propose_controls(
    session: &Arc<Session>,
    uuid: String,
    group: Option<String>,
    ranges: Vec<(i64, u32, u32)>,
    part_size: usize,
    cancel: Cancel,
    progress: impl Fn(Progress) -> bool + Send + Sync + Clone + 'static,
) -> Result<(), WorkError> {
    if ranges.is_empty() {
        return Err(WorkError::Invalid("choose the pages to read".to_owned()));
    }

    let (model, instructions, group_title) = {
        let (uuid, group) = (uuid.clone(), group.clone());
        on_db(session, move |db| {
            let catalog = catalogs::get(db, &uuid)?;
            let title = match &group {
                Some(id) => Some(
                    known_groups(&catalog)
                        .into_iter()
                        .find(|(known, _)| known == id)
                        .map(|(_, title)| title)
                        .ok_or(OscalError::NoSuchObject(id.clone()))?,
                ),
                None => None,
            };
            let (model, instructions) = model_and_instructions(db, Task::CatalogSection)?;

            Ok((model, instructions, title))
        })
        .await?
    };
    let model = Arc::new(model);
    let parts = parts_of(session, ranges, part_size).await?;
    let count = parts.len();

    let stopped = 'parts: {
        for (index, (part, document)) in parts.into_iter().enumerate() {
            if cancel.is_cancelled() {
                break 'parts true;
            }

            // The controls known: those of the catalog and those proposed
            // already, so that a part does not propose them again.
            let known = {
                let uuid = uuid.clone();
                on_db(session, move |db| {
                    let mut known = known_controls(&catalogs::get(db, &uuid)?);

                    for proposal in db_evaluation_support::read::proposals(db, &uuid)? {
                        if let (Some(id), Some(title)) = (proposal.content.get("id"), proposal.content.get("title")) {
                            known.push((
                                id.as_str().unwrap_or_default().to_owned(),
                                title.as_str().unwrap_or_default().to_owned(),
                            ));
                        }
                    }

                    Ok(known)
                })
                .await?
            };
            let turns = completions::single(catalog_section::prompt(
                &instructions,
                &document,
                &part,
                group_title.as_deref(),
                &known,
            ));

            let started = Progress::Part {
                index: index + 1,
                count,
                document: document.fields.title.clone(),
                first_page: part.first_page,
                last_page: part.last_page,
            };

            if !progress(started) {
                break 'parts true;
            }

            let Some(answer) = ask(session, &model, Task::CatalogSection, &turns, &cancel, &progress).await else {
                break 'parts true;
            };

            let Some(answer) = answer else {
                continue;
            };

            match catalog_section::read(&answer, &part) {
                Ok(section) => {
                    let (uuid, group, name) = (uuid.clone(), group.clone(), model.name.clone());
                    let dropped = section.dropped.clone();
                    let added = on_db(session, move |db| {
                        for proposed in &section.controls {
                            let quotes_ = checked_quotes(db, &part, &proposed.quotes);
                            let content = serde_json::to_value(&proposed.control)
                                .map_err(|error| WorkError::Invalid(error.to_string()))?;

                            db_evaluation_support::create::proposal(
                                db,
                                &NewProposal {
                                    document_uuid: &uuid,
                                    kind: ProposalKind::Control,
                                    parent_id: group.as_deref(),
                                    content: &content,
                                    quotes: &quotes_,
                                    note: &proposed.note,
                                    model: &name,
                                },
                            )?;
                        }

                        Ok(section.controls.len())
                    })
                    .await?;

                    progress(Progress::Proposed { added, dropped });
                }
                Err(reason) => {
                    progress(Progress::Failed(reason));
                }
            }
        }

        false
    };

    progress(Progress::End { stopped });

    Ok(())
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "building_tests.rs"]
mod tests;
