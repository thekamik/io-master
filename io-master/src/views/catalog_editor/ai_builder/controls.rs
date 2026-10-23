//! The controls of a group of a catalog proposed by the AI from the pages
//! of the group, part by part.

use std::sync::Arc;

use tauri::ipc::Channel;
use tokio::sync::watch;

use crate::ai::api::completions;
use crate::ai::api::roles::Model;
use crate::ai::tasks::catalog_section::{self, Section};
use crate::app::error::UiError;
use crate::db::evaluation_support as db_evaluation_support;
use crate::db::evaluation_support::create::NewProposal;
use crate::db::oscal as db_oscal;
use crate::db::oscal::error::OscalError;
use crate::db::user::database::UserDb;
use crate::installation::session::Session;
use crate::model::ai::Task;
use crate::model::evaluation_support::ProposalKind;
use crate::model::standards::{Document, Part};

use super::steps::{self, Asked};
use super::view::{BuildEvent, PageRange};

/// The most ranges of pages read at once.
pub const MAX_RANGES: usize = 50;

/// Reads the pages `ranges` of a group of a standard, part by part (at
/// most `part_size` characters each), and proposes the controls of group
/// `group` (`None`: the top) of catalog `uuid`, sending the events to
/// `events`; `stop` ends the work after the step that runs.
pub async fn propose(
    session: &Session,
    uuid: &str,
    group: Option<&str>,
    ranges: &[PageRange],
    part_size: usize,
    stop: &watch::Receiver<bool>,
    events: &Channel<BuildEvent>,
) -> Result<(), UiError> {
    ensure!(!ranges.is_empty(), UiError::Invalid("choose the pages to read".to_owned()));
    ensure!(ranges.len() <= MAX_RANGES, UiError::Invalid(format!("at most {MAX_RANGES} ranges of pages")));

    let Prepared { model, instructions, title, parts } = prepare(&session.db(), uuid, group, ranges, part_size)?;
    let model = Arc::new(model);
    let mut stopped = false;

    for (index, (part, document)) in parts.iter().enumerate() {
        let known = known_controls(&session.db(), uuid)?;
        let turns =
            completions::single(catalog_section::prompt(&instructions, document, part, title.as_deref(), &known));

        if *stop.borrow() || events.send(steps::part_event(index, parts.len(), part, document)).is_err() {
            stopped = true;
            break;
        }

        let answer = match steps::ask(session, &model, Task::CatalogSection, &turns, stop, events).await {
            Asked::Answer(answer) => answer,
            Asked::Failed => continue,
            Asked::Stopped => {
                stopped = true;
                break;
            }
        };

        let event = match catalog_section::read(&answer, part) {
            Ok(section) => {
                let added = store(&session.db(), uuid, group, &model.name, part, &section)?;
                BuildEvent::Proposed { added, dropped: section.dropped }
            }
            Err(message) => BuildEvent::Failed { message },
        };

        if events.send(event).is_err() {
            stopped = true;
            break;
        }
    }

    ensure!(
        events.send(BuildEvent::End { stopped }).is_ok(),
        UiError::Failed("the window can no longer be reached".to_owned())
    );

    Ok(())
}

/// What the work needs before its first question.
struct Prepared {
    model: Model,
    instructions: String,
    /// The title of the group read; `None`: the top of the catalog.
    title: Option<String>,
    /// The parts of the pages, each with its document.
    parts: Vec<(Part, Document)>,
}

fn prepare(
    db: &UserDb,
    uuid: &str,
    group: Option<&str>,
    ranges: &[PageRange],
    part_size: usize,
) -> Result<Prepared, UiError> {
    let catalog = db_oscal::read::catalog(db, uuid)?;

    let title = match group {
        None => None,
        Some(id) => Some(group_title(&catalog.group_titles()?, id)?),
    };

    let (model, instructions) = steps::model_and_instructions(db, Task::CatalogSection)?;

    let mut triples = Vec::with_capacity(ranges.len());

    for range in ranges {
        triples.push((range.document_id, range.first_page, range.last_page));
    }

    Ok(Prepared { model, instructions, title, parts: steps::parts_with_documents(db, &triples, part_size)? })
}

/// The title of group `id`.
fn group_title(groups: &[(String, String)], id: &str) -> Result<String, UiError> {
    for (known, title) in groups {
        if known == id {
            return Ok(title.clone());
        }
    }

    Err(OscalError::NoSuchObject(id.to_owned()).into())
}

/// The controls known (id, title): those of the catalog and those proposed
/// already, so that a part does not propose them again.
fn known_controls(db: &UserDb, uuid: &str) -> Result<Vec<(String, String)>, UiError> {
    let catalog = db_oscal::read::catalog(db, uuid)?;
    let mut known = Vec::new();

    for (_, control) in catalog.all_controls()? {
        known.push((control.id.clone(), control.title.clone()));
    }

    for proposal in db_evaluation_support::read::proposals(db, uuid)? {
        let id = proposal.content.get("id").and_then(serde_json::Value::as_str);
        let title = proposal.content.get("title").and_then(serde_json::Value::as_str);

        if let (Some(id), Some(title)) = (id, title) {
            known.push((id.to_owned(), title.to_owned()));
        }
    }

    Ok(known)
}

/// Stores the controls of `section` as proposals of `model` for group
/// `group` of catalog `uuid`; returns how many.
fn store(
    db: &UserDb,
    uuid: &str,
    group: Option<&str>,
    model: &str,
    part: &Part,
    section: &Section,
) -> Result<usize, UiError> {
    for proposed in &section.controls {
        let quotes = steps::checked_quotes(db, part, &proposed.quotes);

        let content = match serde_json::to_value(&proposed.control) {
            Ok(content) => content,
            Err(error) => return Err(UiError::Failed(error.to_string())),
        };

        let proposal = NewProposal {
            document_uuid: uuid,
            kind: ProposalKind::Control,
            parent_id: group,
            content: &content,
            quotes: &quotes,
            note: &proposed.note,
            model,
        };

        db_evaluation_support::create::proposal(db, &proposal)?;
    }

    Ok(section.controls.len())
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "controls_tests.rs"]
mod tests;
