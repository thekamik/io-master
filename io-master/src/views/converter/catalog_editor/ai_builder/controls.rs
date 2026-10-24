//! The controls of a group of a catalog proposed by the AI from the pages
//! of the group, part by part.

use std::sync::Arc;

use tauri::ipc::Channel;
use tokio::sync::watch;

use crate::ai::api::completions;
use crate::ai::api::roles::Model;
use crate::ai::tasks::catalog_section::{self, Section};
use crate::app::error::AppError;
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
) -> Result<(), AppError> {
    ensure!(!ranges.is_empty(), AppError::Invalid("choose the pages to read".to_owned()));
    ensure!(ranges.len() <= MAX_RANGES, AppError::Invalid(format!("at most {MAX_RANGES} ranges of pages")));

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
        AppError::Failed("the window can no longer be reached".to_owned())
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
) -> Result<Prepared, AppError> {
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
fn group_title(groups: &[(String, String)], id: &str) -> Result<String, AppError> {
    for (known, title) in groups {
        if known == id {
            return Ok(title.clone());
        }
    }

    Err(OscalError::NoSuchObject(id.to_owned()).into())
}

/// The controls known (id, title): those of the catalog and those proposed
/// already, so that a part does not propose them again.
fn known_controls(db: &UserDb, uuid: &str) -> Result<Vec<(String, String)>, AppError> {
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
) -> Result<usize, AppError> {
    for proposed in &section.controls {
        let quotes = steps::checked_quotes(db, part, &proposed.quotes);

        let content = match serde_json::to_value(&proposed.control) {
            Ok(content) => content,
            Err(error) => return Err(AppError::Failed(error.to_string())),
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

#[cfg(test)]
mod tests {
    //! Tests of `controls.rs`, with a fake AI service; the acceptance of the
    //! proposals is tested in `proposals/accept.rs`.

    use super::*;

    use serde_json::json;

    use crate::ai::api::test_support::{FakeAnswer, FakeService, completions_events};
    use crate::model::oscal::catalog::edit::Place;
    use crate::views::converter::catalog_editor::test_support::{collector, ready};

    #[tokio::test]
    async fn the_controls_of_a_group_are_proposed_and_wait() {
        let answer = json!({"controls": [{
        "identifier": "ADV_FSP.4", "title": "Complete functional specification",
        "statement": [{"identifier": "ADV_FSP.4.1D", "text": "The developer shall provide a functional specification."}],
        "objectives": [{"identifier": "ADV_FSP.4.1E", "text": "The evaluator shall confirm the information.", "method": "EXAMINE"}],
        "quotes": [{"page": 2, "text": "The developer shall provide a functional specification."}],
        "note": "From page 2."
    }]})
    .to_string();
        let service = FakeService::start(FakeAnswer::Stream(completions_events(&[&answer]))).await;
        let (test, uuid, source) = ready(&service).await;
        let session = test.state.session().unwrap();

        {
            let db = session.db();
            let mut catalog = db_oscal::read::catalog(&db, &uuid).unwrap();
            catalog
                .add_group(&Place::Top, serde_json::from_value(json!({"id": "ADV", "title": "Development"})).unwrap())
                .unwrap();
            db_oscal::update::catalog(&db, catalog).unwrap();
        }

        let (_, channel) = collector();
        let ranges = [PageRange { document_id: source, first_page: 2, last_page: 3 }];

        propose(&session, &uuid, Some("ADV"), &ranges, 0, &test.state.start_work(), &channel).await.unwrap();

        let db = session.db();
        let waiting = db_evaluation_support::read::proposals(&db, &uuid).unwrap();
        assert_eq!(waiting.len(), 1);
        assert_eq!(waiting[0].note, "From page 2.");
        assert!(waiting[0].quotes[0].found);
        // Nothing enters the catalog before the evaluator accepts it.
        assert!(db_oscal::read::catalog(&db, &uuid).unwrap().all_controls().unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_group_that_does_not_exist_and_no_pages_are_refused() {
        let service = FakeService::start(FakeAnswer::Stream(completions_events(&["{}"]))).await;
        let (test, uuid, source) = ready(&service).await;
        let session = test.state.session().unwrap();
        let (_, channel) = collector();
        let ranges = [PageRange { document_id: source, first_page: 1, last_page: 1 }];

        let missing = propose(&session, &uuid, Some("NOT_THERE"), &ranges, 0, &test.state.start_work(), &channel).await;
        let none = propose(&session, &uuid, None, &[], 0, &test.state.start_work(), &channel).await;

        assert!(matches!(missing, Err(AppError::Invalid(_))));
        assert!(matches!(none, Err(AppError::Invalid(_))));
    }
}
