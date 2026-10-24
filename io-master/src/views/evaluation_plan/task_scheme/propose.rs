//! The AI proposes the inputs of the scheme of a task and corrections of
//! its blocks (step 51.3 of docs/todo.md): one question of the task
//! `task_scheme` through `ai/api/`, written into the journal of the AI. The
//! proposals left undecided are removed first; what the AI proposes is
//! stored as proposals (dashed blocks, corrections beside their blocks),
//! which the planner accepts or rejects. Progress goes to the page as events.

use std::sync::Arc;

use tauri::ipc::Channel;
use tokio::sync::watch;

use crate::ai::api::completions::{self, Outcome};
use crate::ai::api::roles::{self, Model};
use crate::ai::api::tasks;
use crate::ai::tasks::task_scheme as scheme_question;
use crate::app::error::AppError;
use crate::db::evaluation_project as db_evaluation_project;
use crate::db::oscal as db_oscal;
use crate::db::oscal::ids::new_uuid;
use crate::db::user::database::UserDb;
use crate::installation::session::Session;
use crate::model::ai::{Role, Task};
use crate::model::task_scheme::{self, Block, BlockKind};
use crate::model::text::Invalid;

use super::view::ProposalEvent;

/// How often the page learns how much the model has written, in characters.
const WRITING_EVERY: usize = 400;

/// Asks the AI about the scheme of task `task` and stores its proposals;
/// the events go to `events`, `stop` ends the question.
///
/// Called by: `task_scheme_propose` (`evaluation_plan.rs`).
/// Calls: [`question`], [`ask`], [`store`].
pub async fn propose(
    session: &Session,
    task: &str,
    stop: &watch::Receiver<bool>,
    events: &Channel<ProposalEvent>,
) -> Result<(), AppError> {
    let (model, text, blocks) = question(&session.db(), task)?;
    let answer = ask(session, model, text, stop, events).await;
    let mut stopped = answer.is_none() && *stop.borrow();

    if let Some(answer) = answer {
        let event = match store(&session.db(), task, &answer, &blocks)? {
            Ok(event) => event,
            Err(message) => ProposalEvent::Failed { message },
        };

        stopped = events.send(event).is_err();
    }

    ensure!(
        events.send(ProposalEvent::End { stopped }).is_ok(),
        AppError::Failed("the window can no longer be reached".to_owned())
    );

    Ok(())
}

/// Removes the proposals left undecided in task `task` and makes the
/// question: the requirement from its catalog, the scope of the project, the
/// blocks of the scheme. Gives the model, the question and the blocks.
///
/// Called by: [`propose`].
/// Calls: `db_evaluation_project::delete::proposals`, `read::task`,
/// `read::project`, `read::blocks`, `db_oscal::read::catalog`, `roles::model`,
/// `tasks::instructions`, `scheme_question::prompt`.
fn question(db: &UserDb, task: &str) -> Result<(Model, String, Vec<Block>), AppError> {
    let Some((project, found)) = db_evaluation_project::read::task(db, task)? else {
        return Err(AppError::Invalid("the task does not exist".to_owned()));
    };
    let Some(project) = db_evaluation_project::read::project(db, &project)? else {
        return Err(AppError::Invalid("the project does not exist".to_owned()));
    };

    db_evaluation_project::delete::proposals(db, task)?;
    let blocks = db_evaluation_project::read::blocks(db, task)?;
    let catalog = db_oscal::read::catalog(db, &found.catalog_uuid)?;

    let control = match catalog.locate(&found.control_id)? {
        Some(location) => catalog.control_at(&location),
        None => None,
    };
    let Some(control) = control else {
        return Err(AppError::Invalid("the catalog has no such requirement".to_owned()));
    };

    let label = control.label().unwrap_or(&control.id);
    let instructions = tasks::instructions(db, Task::Scheme)?.unwrap_or_default();
    let text = scheme_question::prompt(&instructions, label, control, &project.fields.description, &blocks)
        .map_err(AppError::Invalid)?;

    Ok((roles::model(db, Role::TextAnalysis)?, text, blocks))
}

/// Asks `model` the question `text`, telling the page how much it has
/// written; gives the answer, or `None` when the question failed (the page
/// was told why) or was stopped.
///
/// Called by: [`propose`].
/// Calls: `completions::ask` (which writes the journal), `completions::outcome`.
async fn ask(
    session: &Session,
    model: Model,
    text: String,
    stop: &watch::Receiver<bool>,
    events: &Channel<ProposalEvent>,
) -> Option<String> {
    // A page that is gone misses the progress; the answer is stored all the
    // same, and the end of the work sees the page is gone.
    let writing = |characters| {
        let _page_reached = events.send(ProposalEvent::Writing { characters }).is_ok();
    };
    let turns = completions::single(text);
    let asked = completions::ask(
        &session.db_handle(),
        &Arc::new(model),
        Task::Scheme,
        &turns,
        stop,
        completions::counting(WRITING_EVERY, writing),
    )
    .await;

    if let Err(error) = &asked.journal {
        let message = format!("the question was not written into the journal of the AI: {error}");
        let _page_reached = events.send(ProposalEvent::Failed { message }).is_ok();
    }

    match completions::outcome(&asked.reply, stop) {
        Outcome::Answer(answer) => Some(answer),
        Outcome::Stopped => None,
        Outcome::Failed(message) => {
            let _page_reached = events.send(ProposalEvent::Failed { message }).is_ok();
            None
        }
    }
}

/// Reads `answer` for the scheme of `blocks` and stores its proposals in
/// task `task`: each input a dashed block on the left, connected to the
/// requirement; each correction beside its block. What breaks the rules of
/// a block is dropped with the reason. Gives the event of what was stored,
/// or why the answer could not be read.
///
/// Called by: [`propose`].
/// Calls: `scheme_question::read`, [`store_inputs`],
/// `task_scheme::check_correction`, `db_evaluation_project::create::correction`.
fn store(db: &UserDb, task: &str, answer: &str, blocks: &[Block]) -> Result<Result<ProposalEvent, String>, AppError> {
    let read = match scheme_question::read(answer, blocks) {
        Ok(read) => read,
        Err(message) => return Ok(Err(message)),
    };
    let mut dropped = read.dropped;
    let inputs = store_inputs(db, task, read.inputs, blocks, &mut dropped)?;
    let mut corrections = 0;

    for correction in read.corrections {
        let Some(block) = blocks.iter().find(|block| block.uuid == correction.block) else {
            dropped.push("a correction of a block that is gone".to_owned());
            continue;
        };

        if let Err(error) = task_scheme::check_correction(block.kind, &correction) {
            dropped.push(format!("the correction of \"{}\": {error}", block.fields.name));
            continue;
        }

        db_evaluation_project::create::correction(db, task, &correction)?;
        corrections += 1;
    }

    Ok(Ok(ProposalEvent::Proposed { inputs, corrections, dropped }))
}

/// Stores the proposed `inputs` (fields and reason) as dashed blocks of task
/// `task`, placed one below another left of the requirement and connected to
/// it; an input that breaks the rules, or finds no room, goes to `dropped`.
/// Gives how many were stored.
///
/// Called by: [`store`].
/// Calls: `task_scheme::normalized`, [`check_input`], `next_input_place`,
/// `db_evaluation_project::create::block` and `connection`.
fn store_inputs(
    db: &UserDb,
    task: &str,
    inputs: Vec<(task_scheme::BlockFields, String)>,
    blocks: &[Block],
    dropped: &mut Vec<String>,
) -> Result<usize, AppError> {
    let Some(requirement) = blocks.iter().find(|block| block.kind == BlockKind::Requirement) else {
        return Err(AppError::Invalid("the task has no block of its requirement".to_owned()));
    };
    let requirement = requirement.uuid.clone();
    let mut placed = blocks.to_vec();
    let mut stored = 0;

    for (fields, reason) in inputs {
        let fields = task_scheme::normalized(&fields);
        if let Err(error) = check_input(&fields, &reason, &placed) {
            dropped.push(format!("the input \"{}\": {error}", fields.name));
            continue;
        }

        let (x, y) = task_scheme::next_input_place(&placed);
        let block = Block { uuid: new_uuid()?, kind: BlockKind::Input, x, y, fields, proposal: Some(reason) };

        db_evaluation_project::create::block(db, task, &block)?;
        db_evaluation_project::create::connection(db, task, &block.uuid, &requirement)?;
        placed.push(block);
        stored += 1;
    }

    Ok(stored)
}

/// Checks a proposed input: the rules of the fields of an input, its reason,
/// and room for one more input among `placed`.
///
/// Called by: [`store_inputs`].
/// Calls: `task_scheme::check_fields`, `check_reason`, `check_new_block`.
fn check_input(fields: &task_scheme::BlockFields, reason: &str, placed: &[Block]) -> Result<(), Invalid> {
    task_scheme::check_fields(BlockKind::Input, fields)?;
    task_scheme::check_reason(reason)?;
    task_scheme::check_new_block(placed, BlockKind::Input)
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
    use crate::model::task_scheme::{BlockFields, new_scheme};

    /// "anna" logged in, her AI the fake `service`, a project of the basic
    /// catalog with the task of requirement s1.1.1 and one output; gives the
    /// state and the task.
    async fn ready(service: &FakeService) -> (TestState, String) {
        let test = logged_in().await;

        {
            let session = test.state.session().unwrap();
            let db = session.db();
            let id = connections::create(&db, &connection_to(service)).unwrap().id;
            connections::set_key(&db, id, &ApiKey::new("sk-test".to_owned()).unwrap()).unwrap();

            let catalog = crate::test_support::db::basic_catalog();
            crate::db::oscal::create::document(&db, &catalog).unwrap();
            let fields = ProjectFields { title: "P".to_owned(), description: "A smart card.".to_owned() };
            db_evaluation_project::create::project(&db, "p", &fields, &[catalog.uuid().to_owned()]).unwrap();
            let task = EvaluationTask {
                uuid: "t".to_owned(),
                catalog_uuid: catalog.uuid().to_owned(),
                control_id: "s1.1.1".to_owned(),
            };
            let mut output = BlockFields::empty();
            output.name = "pETR".to_owned();
            let scheme = new_scheme("r", &[("o".to_owned(), output)]).unwrap();
            db_evaluation_project::create::task(&db, "p", &task, &scheme).unwrap();
        }

        (test, "t".to_owned())
    }

    /// A channel that collects the events it gets, as JSON.
    fn collector() -> (Arc<Mutex<Vec<Value>>>, Channel<ProposalEvent>) {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&events);

        let channel = Channel::new(move |body: InvokeResponseBody| {
            sink.lock().unwrap().push(body.deserialize::<Value>().unwrap());
            Ok(())
        });

        (events, channel)
    }

    fn answer() -> String {
        json!({
            "inputs": [
                { "name": "Roles document", "evidence": "The list of roles", "instructions": "Check owners.", "reason": "The statement." },
                { "name": "two\nlines", "evidence": "", "instructions": "", "reason": "" }
            ],
            "corrections": [
                { "id": "requirement", "name": "", "evidence": "", "instructions": "Check every role has an owner.", "reason": "Narrower." },
                { "id": "output-1", "name": "pETR 2" }
            ]
        })
        .to_string()
    }

    #[tokio::test]
    async fn the_proposals_of_the_ai_are_stored_dashed_and_beside_their_blocks() {
        let service = FakeService::start(FakeAnswer::Stream(completions_events(&[&answer()]))).await;
        let (test, task) = ready(&service).await;
        let session = test.state.session().unwrap();
        let (events, channel) = collector();

        propose(&session, &task, &test.state.start_work(), &channel).await.unwrap();

        let db = session.db();
        let blocks = db_evaluation_project::read::blocks(&db, &task).unwrap();
        let proposed: Vec<&Block> = blocks.iter().filter(|block| block.proposal.is_some()).collect();
        assert_eq!(proposed.len(), 1);
        assert_eq!(proposed[0].fields.name, "Roles document");
        assert_eq!(db_evaluation_project::read::connections(&db, &task).unwrap().len(), 2);
        assert_eq!(db_evaluation_project::read::correction(&db, "r").unwrap().unwrap().reason, "Narrower.");

        let events = events.lock().unwrap();
        let proposed_event = &events[events.len() - 2];
        assert_eq!((proposed_event["inputs"].clone(), proposed_event["corrections"].clone()), (json!(1), json!(1)));
        assert_eq!(proposed_event["dropped"].as_array().unwrap().len(), 2);
        assert_eq!(events.last().unwrap(), &json!({ "event": "end", "stopped": false }));

        // The question is in the journal of the AI, with the requirement in it.
        assert_eq!(crate::db::ai::journal::read::last(&db, 5).unwrap()[0].task, "task_scheme");
        let asked = service.received()[0].body.to_string();
        assert!(asked.contains("A smart card."));
    }

    #[tokio::test]
    async fn a_new_question_removes_the_proposals_left_undecided() {
        let service = FakeService::start(FakeAnswer::Stream(completions_events(&[&answer()]))).await;
        let (test, task) = ready(&service).await;
        let session = test.state.session().unwrap();
        let (_, channel) = collector();

        propose(&session, &task, &test.state.start_work(), &channel).await.unwrap();
        propose(&session, &task, &test.state.start_work(), &channel).await.unwrap();

        let blocks = db_evaluation_project::read::blocks(&session.db(), &task).unwrap();
        assert_eq!(blocks.iter().filter(|block| block.proposal.is_some()).count(), 1);
    }

    #[tokio::test]
    async fn an_answer_without_json_is_told_and_stores_nothing() {
        let service = FakeService::start(FakeAnswer::Stream(completions_events(&["I cannot help."]))).await;
        let (test, task) = ready(&service).await;
        let session = test.state.session().unwrap();
        let (events, channel) = collector();

        propose(&session, &task, &test.state.start_work(), &channel).await.unwrap();

        assert_eq!(db_evaluation_project::read::blocks(&session.db(), &task).unwrap().len(), 2);
        assert_eq!(events.lock().unwrap()[0]["event"], "failed");
    }
}
