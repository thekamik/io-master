//! The AI finds the table of contents of a document of a standard (step
//! 50.2 of docs/history.md): it reads the first pages (`FIRST_PAGES`) and names
//! the pages of the table of contents, which are kept for the conversion and
//! shown to the evaluator to confirm. The question goes through `ai/api/`
//! into the journal of the AI, as every question.

use std::sync::Arc;

use tauri::ipc::Channel;
use tokio::sync::watch;

use crate::ai::api::completions;
use crate::ai::tasks::catalog_contents::{self, FIRST_PAGES};
use crate::app::error::AppError;
use crate::db_old::conversion as db_conversion;
use crate::db_old::standards as db_standards;
use crate::installation::session::Session;
use crate::model_old::ai::Task;
use crate::model_old::conversion::Pages;

use super::super::ai_builder::parts::PART_RANGE;
use super::super::ai_builder::steps::{self, Asked};
use super::super::ai_builder::view::BuildEvent;

/// Reads the first pages of document `document_id` and keeps the pages of
/// its table of contents for the conversion of `uuid`; gives them, or
/// `None` when the AI found none. The events go to `events`; `stop` ends the
/// work after the question that runs.
///
/// Called by: `conversion_wizard_find_contents` (`catalog_editor.rs`).
/// Calls: `steps::model_and_instructions`, `steps::parts_with_documents`,
/// `catalog_contents::prompt` and `read`, `steps::ask`,
/// `db_conversion::update::contents`.
pub async fn find(
    session: &Session,
    uuid: &str,
    document_id: i64,
    stop: &watch::Receiver<bool>,
    events: &Channel<BuildEvent>,
) -> Result<Option<Pages>, AppError> {
    let (model, instructions, parts) = {
        let db = session.db();

        ensure!(
            db_conversion::read::conversion(&db, uuid)?.is_some(),
            AppError::Invalid("the catalog is not in conversion".to_owned())
        );

        let Some(document) = db_standards::read::document(&db, document_id)? else {
            return Err(AppError::Invalid("the document of the standard does not exist".to_owned()));
        };

        let last = FIRST_PAGES.min(document.page_count);
        let (model, instructions) = steps::model_and_instructions(&db, Task::CatalogContents)?;

        (model, instructions, steps::parts_with_documents(&db, &[(document_id, 1, last)], PART_RANGE.1)?)
    };

    let model = Arc::new(model);
    let mut stopped = false;
    let mut found = None;

    for (index, (part, document)) in parts.iter().enumerate() {
        if *stop.borrow() || events.send(steps::part_event(index, parts.len(), part, document)).is_err() {
            stopped = true;
            break;
        }

        let turns = completions::single(catalog_contents::prompt(&instructions, document, part));

        let answer = match steps::ask(session, &model, Task::CatalogContents, &turns, stop, events).await {
            Asked::Answer(answer) => answer,
            Asked::Failed => continue,
            Asked::Stopped => {
                stopped = true;
                break;
            }
        };

        match catalog_contents::read(&answer, part) {
            Ok(Some((first, last))) => {
                let pages = Pages { document_id, first, last };

                db_conversion::update::contents(&session.db(), uuid, pages)?;
                found = Some(pages);
                break;
            }
            Ok(None) => {}
            Err(message) => {
                if events.send(BuildEvent::Failed { message }).is_err() {
                    stopped = true;
                    break;
                }
            }
        }
    }

    ensure!(
        events.send(BuildEvent::End { stopped }).is_ok(),
        AppError::Failed("the window can no longer be reached".to_owned())
    );

    Ok(found)
}

#[cfg(test)]
mod tests {
    //! Tests of `contents.rs`: the AI is a fake service of the tests that
    //! answers with a fixed text (`ai/api/test_support.rs`).

    use super::*;

    use serde_json::json;

    use crate::ai::api::test_support::{FakeAnswer, FakeService, completions_events};
    use crate::views::converter::catalog_editor::test_support::{collector, ready};

    #[tokio::test]
    async fn the_pages_the_ai_names_are_kept_for_the_conversion() {
        let answer = json!({ "first_page": 1, "last_page": 1 }).to_string();
        let service = FakeService::start(FakeAnswer::Stream(completions_events(&[&answer]))).await;
        let (test, uuid, source) = ready(&service).await;
        let session = test.state.session().unwrap();
        let (events, channel) = collector();

        let found = find(&session, &uuid, source, &test.state.start_work(), &channel).await.unwrap();

        assert_eq!(found, Some(Pages { document_id: source, first: 1, last: 1 }));
        assert_eq!(db_conversion::read::conversion(&session.db(), &uuid).unwrap().unwrap().contents, found);
        assert_eq!(events.lock().unwrap().last().unwrap(), &json!({ "event": "end", "stopped": false }));

        // The question is in the journal of the AI.
        assert_eq!(crate::db_old::ai::journal::read::last(&session.db(), 5).unwrap()[0].task, "catalog_contents");
    }

    #[tokio::test]
    async fn pages_the_ai_made_up_are_refused() {
        let answer = json!({ "first_page": 40, "last_page": 44 }).to_string();
        let service = FakeService::start(FakeAnswer::Stream(completions_events(&[&answer]))).await;
        let (test, uuid, source) = ready(&service).await;
        let session = test.state.session().unwrap();
        let (events, channel) = collector();

        let found = find(&session, &uuid, source, &test.state.start_work(), &channel).await.unwrap();

        assert_eq!(found, None);
        assert!(events.lock().unwrap().iter().any(|event| event["event"] == "failed"));
    }
}
