//! The AI finds the elements of the other catalogs of a project related to
//! the requirement of a task (step 51.4 of docs/history.md): the work units of
//! the CEM, the test cases of a test specification, ... It reads the
//! requirement and the elements not related yet, part by part
//! (`ai/tasks/candidates.rs`), one question of the task `related_elements`
//! per part through `ai/api/`; what it chooses is stored as proposed related
//! elements with their reasons, which the planner accepts or rejects. The
//! proposals left undecided are removed first.

use std::sync::Arc;

use tauri::ipc::Channel;
use tokio::sync::watch;

use crate::ai::api::roles::{self, Model};
use crate::ai::api::tasks;
use crate::ai::tasks::candidates::{self, Candidate, MAX_CANDIDATES};
use crate::ai::tasks::related_elements;
use crate::app::error::AppError;
use crate::db::evaluation_project as db_evaluation_project;
use crate::db::oscal as db_oscal;
use crate::db::user::database::UserDb;
use crate::installation::session::Session;
use crate::model::ai::Task;
use crate::model::evaluation_project::{MAX_LINKS, TaskLink};
use crate::model::oscal::catalog::Control;
use crate::model::task_scheme::check_reason;

use crate::components::ai_work::ask::{self, Asked};
use crate::components::ai_work::view::AiEvent;

/// What a question needs: the model, the instructions, the requirement of
/// the task as the question shows it, and the elements offered.
struct Question {
    model: Arc<Model>,
    instructions: String,
    label: String,
    control: Control,
    candidates: Vec<Candidate>,
}

/// Asks the AI which elements of the other catalogs of its project are
/// related to task `task`, part by part, and stores its proposals; the
/// events go to `events`, `stop` ends the work after the question that runs.
///
/// Called by: `task_links_propose` (`evaluation_plan.rs`).
/// Calls: [`question`], `candidates::parts`, `related_elements::prompt`,
/// `ask::ask`, [`store`].
pub async fn propose(
    session: &Session,
    task: &str,
    stop: &watch::Receiver<bool>,
    events: &Channel<AiEvent>,
) -> Result<(), AppError> {
    let question = question(&session.db(), task)?;
    let parts = candidates::parts(&question.candidates);
    let mut stopped = false;

    for (index, range) in parts.iter().enumerate() {
        let Some(part) = question.candidates.get(range.clone()) else {
            return Err(AppError::Failed("a part of the elements was lost".to_owned()));
        };

        if *stop.borrow() || events.send(AiEvent::Part { index: index + 1, count: parts.len() }).is_err() {
            stopped = true;
            break;
        }

        let text = related_elements::prompt(&question.instructions, &question.label, &question.control, part)
            .map_err(AppError::Invalid)?;

        match ask::ask(session, &question.model, Task::RelatedElements, text, stop, events).await {
            Asked::Answer(answer) => {
                if events.send(store(&session.db(), task, &answer, part)?).is_err() {
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

/// Removes the related elements of task `task` that are only proposed and
/// gathers the question: the requirement of the task and the elements of
/// the other catalogs of its project not related yet (at most
/// `MAX_CANDIDATES`). Refused when the project has no other catalog.
///
/// Called by: [`propose`].
/// Calls: `read::task`, `read::project`, `delete::proposed_links`,
/// `read::task_links`, `db_oscal::read::catalog`, [`offered`], `roles::model`,
/// `tasks::instructions`.
fn question(db: &UserDb, task: &str) -> Result<Question, AppError> {
    let Some((project, found)) = db_evaluation_project::read::task(db, task)? else {
        return Err(AppError::Invalid("the task does not exist".to_owned()));
    };
    let Some(project) = db_evaluation_project::read::project(db, &project)? else {
        return Err(AppError::Invalid("the project does not exist".to_owned()));
    };

    db_evaluation_project::delete::proposed_links(db, task)?;
    let catalog = db_oscal::read::catalog(db, &found.catalog_uuid)?;

    let control = match catalog.locate(&found.control_id)? {
        Some(location) => catalog.control_at(&location),
        None => None,
    };
    let Some(control) = control else {
        return Err(AppError::Invalid("the catalog has no such requirement".to_owned()));
    };

    let mut others = Vec::with_capacity(project.catalogs.len());

    for uuid in &project.catalogs {
        if *uuid != found.catalog_uuid {
            others.push(uuid);
        }
    }

    ensure!(!others.is_empty(), AppError::Invalid("the project has no other catalog to look in".to_owned()));

    Ok(Question {
        model: Arc::new(roles::model(db)?),
        instructions: tasks::instructions(db, Task::RelatedElements)?.unwrap_or_default(),
        label: control.label().unwrap_or(&control.id).to_owned(),
        control: control.clone(),
        candidates: offered(db, &others, &db_evaluation_project::read::task_links(db, task)?)?,
    })
}

/// The elements of catalogs `others` not related to the task yet (`links`),
/// at most `MAX_CANDIDATES`.
///
/// Called by: [`question`].
/// Calls: `db_oscal::read::catalog`, `Catalog::all_controls`, `candidates::candidate`.
fn offered(db: &UserDb, others: &[&String], links: &[TaskLink]) -> Result<Vec<Candidate>, AppError> {
    let mut found = Vec::new();

    for uuid in others {
        let catalog = db_oscal::read::catalog(db, uuid)?;

        for (_, control) in catalog.all_controls()?.into_iter().take(MAX_CANDIDATES) {
            // A short closure: the key of the search.
            let related = links.iter().any(|link| link.catalog_uuid == **uuid && link.control_id == control.id);

            if !related && found.len() < MAX_CANDIDATES {
                found.push(candidates::candidate(uuid, control).map_err(AppError::Invalid)?);
            }
        }
    }

    Ok(found)
}

/// Reads `answer` for `part` and stores the elements chosen as proposed
/// related elements of task `task`; a reason too long, or an element beyond
/// `MAX_LINKS`, is dropped. Gives the event of what was stored, or why the
/// answer could not be read.
///
/// Called by: [`propose`].
/// Calls: `candidates::read_choices`, `check_reason`, `read::task_links`,
/// `db_evaluation_project::create::task_link`.
fn store(db: &UserDb, task: &str, answer: &str, part: &[Candidate]) -> Result<AiEvent, AppError> {
    let choices = match candidates::read_choices(answer, part) {
        Ok(choices) => choices,
        Err(message) => return Ok(AiEvent::Failed { message }),
    };
    let mut dropped = choices.dropped;
    let mut count = db_evaluation_project::read::task_links(db, task)?.len();
    let mut added = 0;

    for (index, reason) in choices.chosen {
        let Some(candidate) = part.get(index) else {
            return Err(AppError::Failed("an element of the part was lost".to_owned()));
        };

        if let Err(error) = check_reason(&reason) {
            dropped.push(format!("{}: {error}", candidate.label));
            continue;
        }

        if count >= MAX_LINKS {
            dropped.push(format!("{}: a task has at most {MAX_LINKS} related elements", candidate.label));
            continue;
        }

        let link = TaskLink {
            catalog_uuid: candidate.catalog_uuid.clone(),
            control_id: candidate.id.clone(),
            reason,
            proposed: true,
        };
        db_evaluation_project::create::task_link(db, task, &link)?;
        count += 1;
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
    use crate::model::oscal::OscalDocument;
    use crate::model::task_scheme::new_scheme;

    /// The UUID of the second catalog of the tests: the basic catalog again,
    /// standing for the methodology.
    const OTHER: &str = "11111111-1111-4111-8111-111111111111";

    /// "anna" logged in, her AI the fake `service`, a project of the basic
    /// catalog and a copy of it (`OTHER`), with requirement s1.1.1 of the
    /// first a task "t"; gives the state.
    async fn ready(service: &FakeService) -> TestState {
        let test = logged_in().await;

        {
            let session = test.state.session().unwrap();
            let db = session.db();
            let id = connections::create(&db, &connection_to(service)).unwrap().id;
            connections::set_key(&db, id, &ApiKey::new("sk-test".to_owned()).unwrap()).unwrap();

            let first = crate::test_support::db::basic_catalog();
            let text = include_str!("../../../../oscal/examples/basic-catalog.json").replace(first.uuid(), OTHER);
            let other = OscalDocument::from_json(&text).unwrap();
            crate::db::oscal::create::document(&db, &first).unwrap();
            crate::db::oscal::create::document(&db, &other).unwrap();
            let fields = ProjectFields { title: "P".to_owned(), description: String::new() };
            db_evaluation_project::create::project(&db, "p", &fields, &[first.uuid().to_owned(), OTHER.to_owned()])
                .unwrap();
            let task = EvaluationTask {
                uuid: "t".to_owned(),
                catalog_uuid: first.uuid().to_owned(),
                control_id: "s1.1.1".to_owned(),
            };
            db_evaluation_project::create::task(&db, "p", &task, &new_scheme("r", &[]).unwrap()).unwrap();
        }

        test
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
    async fn the_elements_of_the_other_catalogs_the_ai_chooses_are_proposed() {
        let answer = json!({ "chosen": [{ "ref": "r1", "reason": "The same requirement." }] }).to_string();
        let service = FakeService::start(FakeAnswer::Stream(completions_events(&[&answer]))).await;
        let test = ready(&service).await;
        let session = test.state.session().unwrap();
        let (events, channel) = collector();

        propose(&session, "t", &test.state.start_work(), &channel).await.unwrap();

        let db = session.db();
        let links = db_evaluation_project::read::task_links(&db, "t").unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(
            (links[0].catalog_uuid.as_str(), links[0].control_id.as_str(), links[0].proposed),
            (OTHER, "s1.1.1", true)
        );
        assert_eq!(events.lock().unwrap().last().unwrap(), &json!({ "event": "end", "stopped": false }));
        assert_eq!(crate::db::ai::journal::read::last(&db, 5).unwrap()[0].task, "related_elements");

        let asked = service.received()[0].body.to_string();
        assert!(asked.contains("Information security roles and responsibilities"));
    }

    #[tokio::test]
    async fn a_project_of_one_catalog_has_nowhere_to_look() {
        let service = FakeService::start(FakeAnswer::Stream(completions_events(&["{}"]))).await;
        let test = ready(&service).await;
        let session = test.state.session().unwrap();
        db_evaluation_project::update::project(
            &session.db(),
            "p",
            &ProjectFields { title: "P".to_owned(), description: String::new() },
            &[crate::test_support::db::basic_catalog().uuid().to_owned()],
        )
        .unwrap();
        let (_, channel) = collector();

        assert!(propose(&session, "t", &test.state.start_work(), &channel).await.is_err());
    }
}
