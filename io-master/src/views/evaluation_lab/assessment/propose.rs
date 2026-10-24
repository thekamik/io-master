//! The agent of a requirement proposes its four results and a verdict (step
//! 52 of docs/todo.md): one question of the task `assessment` (role
//! *formal analysis*) through `ai/api/`, with the requirement, what the
//! planner asks to check and **only the fragments the evaluator accepted**,
//! listed by input (decisions of the user, 2026-10-17). The proposal is kept
//! beside the results; the evaluator accepts or corrects it and sets the
//! verdict (P1).

use std::sync::Arc;

use tauri::ipc::Channel;
use tokio::sync::watch;

use crate::ai::api::roles::{self, Model};
use crate::ai::api::tasks;
use crate::ai::tasks::assessment;
use crate::app::error::AppError;
use crate::components::ai_work::ask::{self, Asked};
use crate::components::ai_work::view::AiEvent;
use crate::db::evaluation as db_evaluation;
use crate::db::user::database::UserDb;
use crate::installation::session::Session;
use crate::model::ai::{Role, Task};
use crate::model::evaluation::check_results;
use crate::model::task_scheme::{BlockKind, check_reason};

use super::super::lab_task::read as lab_task;

/// Asks the agent of the requirement of task `task` for its results and
/// keeps the proposal. The events go to `events`.
///
/// Called by: `assessment_propose` (`evaluation_lab.rs`).
/// Calls: [`question`], `ask::ask`, [`store`].
pub async fn propose(
    session: &Session,
    evaluation: &str,
    task: &str,
    stop: &watch::Receiver<bool>,
    events: &Channel<AiEvent>,
) -> Result<(), AppError> {
    let (model, text) = question(&session.db(), evaluation, task)?;

    let stopped = match ask::ask(session, &model, Task::Assessment, text, stop, events).await {
        Asked::Answer(answer) => {
            let event = store(&session.db(), evaluation, task, &answer)?;

            events.send(event).is_err()
        }
        Asked::Failed => false,
        Asked::Stopped => true,
    };

    ensure!(
        events.send(AiEvent::End { stopped }).is_ok(),
        AppError::Failed("the window can no longer be reached".to_owned())
    );

    Ok(())
}

/// The model and the question: the requirement, the instructions of its
/// block, and the accepted fragments by input with their evidence and page.
///
/// Called by: [`propose`].
/// Calls: `lab_task::task` and `requirement`, `read::blocks`,
/// `read::accepted_fragments`, `read::evidence_of`, `assessment::prompt`.
fn question(db: &UserDb, evaluation: &str, task: &str) -> Result<(Arc<Model>, String), AppError> {
    let (_, requirement, _) = lab_task::requirement(db, &lab_task::task(db, evaluation, task)?)?;
    let blocks = db_evaluation::read::blocks(db, evaluation, task)?;
    let fragments = db_evaluation::read::accepted_fragments(db, evaluation, task)?;
    let mut checks = String::new();
    let mut listed = String::new();

    for block in &blocks {
        if block.kind == BlockKind::Requirement {
            checks.clone_from(&block.fields.instructions);
            continue;
        }

        if block.kind != BlockKind::Input {
            continue;
        }

        let evidence = db_evaluation::read::evidence_of(db, evaluation, &block.uuid)?;
        listed.push_str(&format!("Input \"{}\" (it expects: {}):\n", block.fields.name, block.fields.evidence));

        for (owner, fragment) in &fragments {
            if *owner != block.uuid {
                continue;
            }

            let source =
                evidence.iter().find(|piece| piece.uuid == fragment.evidence).map_or("?", |piece| piece.name.as_str());
            let page = fragment.page.map_or_else(String::new, |page| format!(", page {page}"));

            listed.push_str(&format!("- \"{}\" ({source}{page})\n", fragment.quote));

            if !fragment.note.is_empty() {
                listed.push_str(&format!("  AI note: {}\n", fragment.note));
            }
        }
    }

    let instructions = tasks::instructions(db, Task::Assessment)?.unwrap_or_default();

    Ok((
        Arc::new(roles::model(db, Role::FormalAnalysis)?),
        assessment::prompt(&instructions, &requirement, &checks, &listed),
    ))
}

/// Reads `answer` and keeps it as the proposal for the results of task
/// `task`; the task goes in progress. Gives the event of what was kept, or
/// why the answer could not be read.
///
/// Called by: [`propose`].
/// Calls: `assessment::read`, `check_results`, `check_reason`,
/// `db_evaluation::update::results_proposal`.
fn store(db: &UserDb, evaluation: &str, task: &str, answer: &str) -> Result<AiEvent, AppError> {
    let (results, reason) = match assessment::read(answer) {
        Ok(read) => read,
        Err(message) => return Ok(AiEvent::Failed { message }),
    };

    if let Err(error) = check_results(&results).and_then(|()| check_reason(&reason)) {
        return Ok(AiEvent::Failed { message: error.to_string() });
    }

    db_evaluation::update::results_proposal(db, evaluation, task, &results, &reason)?;
    db_evaluation::update::task_started(db, evaluation, task)?;

    Ok(AiEvent::Proposed { added: 1, corrections: 0, dropped: Vec::new() })
}

#[cfg(test)]
mod tests {
    //! Tests of `propose.rs`: the AI is a fake service of the tests.

    use super::*;

    use serde_json::json;

    use crate::ai::api::test_support::{FakeAnswer, FakeService, completions_events};
    use crate::model::evaluation::{Fragment, Verdict};
    use crate::views::evaluation_lab::test_support::{collector, ready};

    #[tokio::test]
    async fn the_agent_reads_only_the_accepted_fragments_and_its_proposal_is_kept() {
        let answer = json!({ "findings": "Anna owns the keys.", "analysis": "A", "assessment": "S",
                             "vulnerabilities": "V", "verdict": "met", "reason": "All roles have owners." })
        .to_string();
        let service = FakeService::start(FakeAnswer::Stream(completions_events(&[&answer]))).await;
        let test = ready(&service).await;
        let session = test.state.session().unwrap();
        {
            let db = session.db();
            let evidence = db_evaluation::read::evidence_of(&db, "e", "i").unwrap()[0].uuid.clone();
            for (uuid, quote, accepted) in [("f1", "Anna owns the keys", true), ("f2", "Bob owns the audit log", false)]
            {
                let fragment = Fragment {
                    uuid: uuid.to_owned(),
                    evidence: evidence.clone(),
                    page: Some(1),
                    quote: quote.to_owned(),
                    note: String::new(),
                    found: true,
                    accepted,
                    by_ai: true,
                };
                db_evaluation::create::fragment(&db, "e", "t", "i", &fragment).unwrap();
            }
        }
        let (_, channel) = collector();

        propose(&session, "e", "t", &test.state.start_work(), &channel).await.unwrap();

        let (proposed, reason) = db_evaluation::read::results_proposal(&session.db(), "e", "t").unwrap().unwrap();
        assert_eq!((proposed.verdict, reason.as_str()), (Some(Verdict::Met), "All roles have owners."));
        assert_eq!(
            db_evaluation::read::results(&session.db(), "e", "t").unwrap().verdict,
            None,
            "the evaluator decides"
        );

        let asked = service.received()[0].body.to_string();
        assert!(asked.contains("Anna owns the keys"));
        assert!(!asked.contains("Bob owns the audit log"), "a fragment not accepted stays out");
        assert_eq!(crate::db::ai::journal::read::last(&session.db(), 5).unwrap()[0].task, "assessment");
    }
}
