//! The AI proposes the tasks of a master project (step 51.4 of
//! docs/history.md): it reads the scope of the project and the requirements of
//! one of its catalogs that are no task yet, part by part
//! (`ai/tasks/candidates.rs`), one question of the task `requirement_choice`
//! per part through `ai/api/`; the requirements it chooses are stored as
//! proposals with their reasons, which the planner accepts or rejects. The
//! undecided proposals of the catalog are removed first.

use std::sync::Arc;

use tauri::ipc::Channel;
use tokio::sync::watch;

use crate::ai::api::roles::{self, Model};
use crate::ai::api::tasks;
use crate::ai::tasks::candidates::{self, Candidate, MAX_CANDIDATES};
use crate::ai::tasks::knowledge_context;
use crate::ai::tasks::requirement_choice;
use crate::app::error::AppError;
use crate::components::knowledge_agent::find;
use crate::db::evaluation_project as db_evaluation_project;
use crate::db::oscal as db_oscal;
use crate::db::user::database::UserDb;
use crate::installation::session::Session;
use crate::model::ai::Task;
use crate::model::evaluation_project::TaskProposal;
use crate::model::knowledge::AgentPlace;
use crate::model::task_scheme::check_reason;

use crate::components::ai_work::ask::{self, Asked};
use crate::components::ai_work::view::AiEvent;

/// What a question needs: the model, the instructions, the scope of the
/// project, the title of the catalog and the requirements offered.
struct Question {
    model: Arc<Model>,
    instructions: String,
    scope: String,
    catalog_title: String,
    /// The context of the agent of the project (step 57.6); empty without one.
    context: String,
    candidates: Vec<Candidate>,
}

/// Asks the AI which requirements of catalog `catalog` project `project`
/// has to cover, part by part, and stores its proposals; the events go to
/// `events`, `stop` ends the work after the question that runs.
///
/// Called by: `task_proposals_propose` (`evaluation_plan.rs`).
/// Calls: [`question`], `candidates::parts`, `ask::ask`, [`store`].
pub async fn propose(
    session: &Session,
    project: &str,
    catalog: &str,
    stop: &watch::Receiver<bool>,
    events: &Channel<AiEvent>,
) -> Result<(), AppError> {
    let question = question(&session.db(), project, catalog)?;
    let parts = candidates::parts(&question.candidates);
    let mut stopped = false;

    for (index, range) in parts.iter().enumerate() {
        let Some(part) = question.candidates.get(range.clone()) else {
            return Err(AppError::Failed("a part of the requirements was lost".to_owned()));
        };

        if *stop.borrow() || events.send(AiEvent::Part { index: index + 1, count: parts.len() }).is_err() {
            stopped = true;
            break;
        }

        let text = knowledge_context::appended(
            requirement_choice::prompt(&question.instructions, &question.scope, &question.catalog_title, part),
            &question.context,
        );

        match ask::ask(session, &question.model, Task::RequirementChoice, text, stop, events).await {
            Asked::Answer(answer) => {
                let event = store(&session.db(), project, &answer, part)?;

                if events.send(event).is_err() {
                    stopped = true;
                    break;
                }
            }
            Asked::Failed => {}
            Asked::Stopped => {
                stopped = true;
                break;
            }
        }
    }

    ensure!(
        events.send(AiEvent::End { stopped }).is_ok(),
        AppError::Failed("the window can no longer be reached".to_owned())
    );

    Ok(())
}

/// Removes the undecided proposals of catalog `catalog` in project
/// `project` and gathers the question: the requirements of the catalog that
/// are no task of the project yet (at most `MAX_CANDIDATES`).
///
/// Called by: [`propose`].
/// Calls: `read::project`, `read::tasks`, `delete::task_proposals`,
/// `db_oscal::read::catalog`, `Catalog::all_controls`, `candidates::candidate`,
/// `roles::model`, `tasks::instructions`, `find::attached_context` (the context
/// of the agent of the project for the catalog and the scope, step 57.6).
fn question(db: &UserDb, project: &str, catalog: &str) -> Result<Question, AppError> {
    let Some(found) = db_evaluation_project::read::project(db, project)? else {
        return Err(AppError::Invalid("the project does not exist".to_owned()));
    };

    ensure!(
        found.catalogs.contains(&catalog.to_owned()),
        AppError::Invalid("the project does not use this catalog".to_owned())
    );

    let tasks = db_evaluation_project::read::tasks(db, project)?;
    db_evaluation_project::delete::task_proposals(db, project, Some(catalog))?;
    let document = db_oscal::read::catalog(db, catalog)?;
    let mut offered = Vec::new();

    for (_, control) in document.all_controls()?.into_iter().take(MAX_CANDIDATES) {
        // A short closure: the key of the search.
        if tasks.iter().any(|task| task.catalog_uuid == catalog && task.control_id == control.id) {
            continue;
        }

        offered.push(candidates::candidate(catalog, control).map_err(AppError::Invalid)?);
    }

    let words = format!("{} {}", document.metadata.title, found.fields.description);

    Ok(Question {
        model: Arc::new(roles::model(db)?),
        instructions: tasks::instructions(db, Task::RequirementChoice)?.unwrap_or_default(),
        context: find::attached_context(db, AgentPlace::Project, project, &words)?,
        scope: found.fields.description,
        catalog_title: document.metadata.title.clone(),
        candidates: offered,
    })
}

/// Reads `answer` for `part` and stores the requirements chosen as proposals
/// of project `project`; a reason too long is dropped. Gives the event of
/// what was stored, or why the answer could not be read.
///
/// Called by: [`propose`].
/// Calls: `candidates::read_choices`, `check_reason`,
/// `db_evaluation_project::create::task_proposal`.
fn store(db: &UserDb, project: &str, answer: &str, part: &[Candidate]) -> Result<AiEvent, AppError> {
    let choices = match candidates::read_choices(answer, part) {
        Ok(choices) => choices,
        Err(message) => return Ok(AiEvent::Failed { message }),
    };
    let mut dropped = choices.dropped;
    let mut added = 0;

    for (index, reason) in choices.chosen {
        let Some(candidate) = part.get(index) else {
            return Err(AppError::Failed("a requirement of the part was lost".to_owned()));
        };

        if let Err(error) = check_reason(&reason) {
            dropped.push(format!("{}: {error}", candidate.label));
            continue;
        }

        let proposal =
            TaskProposal { catalog_uuid: candidate.catalog_uuid.clone(), control_id: candidate.id.clone(), reason };
        db_evaluation_project::create::task_proposal(db, project, &proposal)?;
        added += 1;
    }

    Ok(AiEvent::Proposed { added, corrections: 0, dropped })
}

#[cfg(test)]
mod tests {
    //! Tests of `propose.rs`: the AI is a fake service of the tests that
    //! answers with a fixed text (`ai/api/test_support.rs`).

    use super::*;

    use std::sync::Mutex;

    use serde_json::{Value, json};
    use tauri::ipc::InvokeResponseBody;

    use crate::ai::api::connections;
    use crate::ai::api::test_support::{FakeAnswer, FakeService, completions as connection_to, completions_events};
    use crate::app::test_support::{TestState, logged_in};
    use crate::model::api_key::ApiKey;
    use crate::model::evaluation_project::{EvaluationTask, ProjectFields};
    use crate::model::task_scheme::new_scheme;

    /// "anna" logged in, her AI the fake `service`, a project of the basic
    /// catalog of NIST with requirement s1.1.1 a task already; gives the
    /// state and the catalog.
    async fn ready(service: &FakeService) -> (TestState, String) {
        let test = logged_in().await;

        let catalog = {
            let session = test.state.session().unwrap();
            let db = session.db();
            let id = connections::create(&db, &connection_to(service)).unwrap().id;
            connections::set_key(&db, id, &ApiKey::new("sk-test".to_owned()).unwrap()).unwrap();

            let document = crate::test_support::db::basic_catalog();
            crate::db::oscal::create::document(&db, &document).unwrap();
            let catalog = document.uuid().to_owned();
            let fields = ProjectFields { title: "P".to_owned(), description: "A smart card.".to_owned() };
            db_evaluation_project::create::project(&db, "p", &fields, std::slice::from_ref(&catalog)).unwrap();
            let task =
                EvaluationTask { uuid: "t".to_owned(), catalog_uuid: catalog.clone(), control_id: "s1.1.1".to_owned() };
            db_evaluation_project::create::task(&db, "p", &task, &new_scheme("r", &[]).unwrap()).unwrap();

            catalog
        };

        (test, catalog)
    }

    /// A channel that collects the events it gets, as JSON.
    fn collector() -> (Arc<Mutex<Vec<Value>>>, Channel<AiEvent>) {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&events);

        let channel = Channel::new(move |body: InvokeResponseBody| {
            sink.lock().unwrap().push(body.deserialize::<Value>().unwrap());
            Ok(())
        });

        (events, channel)
    }

    #[tokio::test]
    async fn the_requirements_the_ai_chooses_are_proposed_and_the_tasks_are_not_offered() {
        let answer =
            json!({ "chosen": [{ "ref": "r1", "reason": "The scope needs it." }, { "ref": "r99" }] }).to_string();
        let service = FakeService::start(FakeAnswer::Stream(completions_events(&[&answer]))).await;
        let (test, catalog) = ready(&service).await;
        let session = test.state.session().unwrap();
        let (events, channel) = collector();

        propose(&session, "p", &catalog, &test.state.start_work(), &channel).await.unwrap();

        let db = session.db();
        let proposals = db_evaluation_project::read::task_proposals(&db, "p").unwrap();
        assert_eq!(proposals.len(), 1);
        assert_eq!(proposals[0].control_id, "s1.1.2", "s1.1.1 is a task and is not offered, so r1 is s1.1.2");
        assert_eq!(proposals[0].reason, "The scope needs it.");

        let events = events.lock().unwrap();
        assert_eq!(events[0], json!({ "event": "part", "index": 1, "count": 1 }));
        assert_eq!(events.last().unwrap(), &json!({ "event": "end", "stopped": false }));
        assert_eq!(crate::db::ai::journal::read::last(&db, 5).unwrap()[0].task, "requirement_choice");

        let asked = service.received()[0].body.to_string();
        assert!(asked.contains("A smart card."));
        assert!(!asked.contains("1.1.1: Information security roles"), "a task is not offered again");
    }

    #[tokio::test]
    async fn a_catalog_the_project_does_not_use_is_refused() {
        let service = FakeService::start(FakeAnswer::Stream(completions_events(&["{}"]))).await;
        let (test, _) = ready(&service).await;
        let session = test.state.session().unwrap();
        let (_, channel) = collector();

        assert!(propose(&session, "p", "other", &test.state.start_work(), &channel).await.is_err());
    }
}
