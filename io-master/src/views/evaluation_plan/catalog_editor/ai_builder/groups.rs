//! The groups of a catalog proposed by the AI from a table of contents
//! (the pages of one document), part by part.

use std::sync::Arc;

use tauri::ipc::Channel;
use tokio::sync::watch;

use crate::ai::api::completions;
use crate::ai::tasks::catalog_outline::{self, Outline};
use crate::app::error::AppError;
use crate::db::evaluation_support as db_evaluation_support;
use crate::db::evaluation_support::create::NewProposal;
use crate::db::oscal as db_oscal;
use crate::db::user::database::UserDb;
use crate::installation::session::Session;
use crate::model::ai::Task;
use crate::model::evaluation_support::ProposalKind;
use crate::model::standards::Part;

use super::steps::{self, Asked};
use super::view::{BuildEvent, PageRange};

/// Reads the table of contents in `range` and proposes the groups of
/// catalog `uuid`, sending the events to `events`; `stop` ends the work
/// after the step that runs.
pub async fn propose(
    session: &Session,
    uuid: &str,
    range: &PageRange,
    stop: &watch::Receiver<bool>,
    events: &Channel<BuildEvent>,
) -> Result<(), AppError> {
    let (model, instructions, parts) = {
        let db = session.db();
        db_oscal::read::catalog(&db, uuid)?;

        let (model, instructions) = steps::model_and_instructions(&db, Task::CatalogOutline)?;
        let ranges = [(range.document_id, range.first_page, range.last_page)];

        (model, instructions, steps::parts_with_documents(&db, &ranges, 0)?)
    };

    let model = Arc::new(model);
    let mut stopped = false;

    for (index, (part, document)) in parts.iter().enumerate() {
        // The groups known now, also those accepted during the work.
        let known = db_oscal::read::catalog(&session.db(), uuid)?.group_titles()?;
        let turns = completions::single(catalog_outline::prompt(&instructions, document, part, &known));

        if *stop.borrow() || events.send(steps::part_event(index, parts.len(), part, document)).is_err() {
            stopped = true;
            break;
        }

        let answer = match steps::ask(session, &model, Task::CatalogOutline, &turns, stop, events).await {
            Asked::Answer(answer) => answer,
            Asked::Failed => continue,
            Asked::Stopped => {
                stopped = true;
                break;
            }
        };

        let event = match catalog_outline::read(&answer, part) {
            Ok(outline) => {
                let added = store(&session.db(), uuid, &model.name, part, &outline)?;
                BuildEvent::Proposed { added, dropped: outline.dropped }
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
        AppError::Failed("the window can no longer be reached".to_owned())
    );

    Ok(())
}

/// Stores the groups of `outline` as proposals of `model` for catalog
/// `uuid`; returns how many.
fn store(db: &UserDb, uuid: &str, model: &str, part: &Part, outline: &Outline) -> Result<usize, AppError> {
    for proposed in &outline.groups {
        let quotes = steps::checked_quotes(db, part, proposed.quote.as_slice());

        let content = match serde_json::to_value(&proposed.group) {
            Ok(content) => content,
            Err(error) => return Err(AppError::Failed(error.to_string())),
        };

        let proposal = NewProposal {
            document_uuid: uuid,
            kind: ProposalKind::Group,
            parent_id: proposed.parent.as_deref(),
            content: &content,
            quotes: &quotes,
            note: "",
            model,
        };

        db_evaluation_support::create::proposal(db, &proposal)?;
    }

    Ok(outline.groups.len())
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "groups_tests.rs"]
pub mod tests;
