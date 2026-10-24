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

#[cfg(test)]
mod tests {
    //! Tests of `groups.rs`: the AI is a fake service of the tests
    //! (`ai/api/test_support.rs`) that answers with a fixed text; the events go
    //! into a channel that collects them (`catalog_editor/test_support.rs`).

    use super::*;

    use serde_json::json;

    use crate::ai::api::test_support::{FakeAnswer, FakeService, completions_events};
    use crate::views::converter::catalog_editor::test_support::{collector, ready};

    #[tokio::test]
    async fn the_groups_of_a_table_of_contents_are_proposed_and_wait() {
        let answer = json!({"groups": [
            {"identifier": "ADV", "title": "Development", "parent": null,
             "quote": {"page": 2, "text": "ADV_FSP.4 Complete functional specification"}},
            {"identifier": "ATE", "title": "Tests", "parent": null, "quote": {"page": 3, "text": "Made up heading"}}
        ]})
        .to_string();
        let service = FakeService::start(FakeAnswer::Stream(completions_events(&[&answer]))).await;
        let (test, uuid, source) = ready(&service).await;
        let session = test.state.session().unwrap();
        let (events, channel) = collector();
        let range = PageRange { document_id: source, first_page: 1, last_page: 3 };

        propose(&session, &uuid, &range, &test.state.start_work(), &channel).await.unwrap();

        let events = events.lock().unwrap().clone();
        assert!(events.contains(&json!({ "event": "proposed", "added": 2, "dropped": [] })), "{events:?}");
        assert_eq!(events.last().unwrap(), &json!({ "event": "end", "stopped": false }));

        let db = session.db();
        let waiting = db_evaluation_support::read::proposals(&db, &uuid).unwrap();
        assert_eq!(waiting.len(), 2);
        assert!(waiting[0].quotes[0].found);
        assert!(!waiting[1].quotes[0].found, "a made up heading is shown as such");
        // Nothing enters the catalog before the evaluator accepts it.
        assert!(db_oscal::read::catalog(&db, &uuid).unwrap().groups.is_empty());

        // The question is in the journal of the AI.
        assert_eq!(crate::db::ai::journal::read::last(&db, 5).unwrap()[0].task, "catalog_outline");
    }

    #[tokio::test]
    async fn stop_ends_the_work_before_a_question() {
        let service = FakeService::start(FakeAnswer::Stream(completions_events(&["{}"]))).await;
        let (test, uuid, source) = ready(&service).await;
        let session = test.state.session().unwrap();
        let (events, channel) = collector();
        let range = PageRange { document_id: source, first_page: 1, last_page: 3 };

        let stop = test.state.start_work();
        test.state.stop_work();
        propose(&session, &uuid, &range, &stop, &channel).await.unwrap();

        assert_eq!(events.lock().unwrap().last().unwrap(), &json!({ "event": "end", "stopped": true }));
        assert!(service.received().is_empty());
    }
}
