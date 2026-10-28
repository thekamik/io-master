//! The agent of a requirement proposes its four results and a verdict (step
//! 52 of docs/history.md): one question of the task `assessment` (role
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
use crate::model_old::ai::Task;
use crate::model_old::evaluation::check_results;
use crate::model_old::task_scheme::{BlockKind, check_reason};

use super::super::lab_task::read as lab_task;
use super::context;

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

/// The model and the question: the whole requirement, the instructions of
/// its block, its related elements word for word (step 57.1), the ticked
/// passages of its agent with knowledge (step 57.4), and the accepted
/// fragments by input with their evidence and page.
///
/// Called by: [`propose`].
/// Calls: `lab_task::task`, `requirement` and `related_text`, `context::question_text`, `read::blocks`,
/// `read::accepted_fragments`, `read::evidence_of`, `assessment::prompt`.
fn question(db: &UserDb, evaluation: &str, task: &str) -> Result<(Arc<Model>, String), AppError> {
    let found = lab_task::task(db, evaluation, task)?;
    let (_, requirement, _) = lab_task::requirement(db, &found)?;
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

    let related = lab_task::related_text(db, evaluation, task)?;
    let context = context::question_text(db, evaluation, &found)?;
    let text = assessment::prompt(&instructions, &requirement, &checks, &related, &context, &listed);

    Ok((Arc::new(roles::model(db)?), text))
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
    use crate::model_old::evaluation::{Fragment, Related, Verdict};
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

    #[tokio::test]
    async fn the_agent_gets_the_whole_requirement_and_its_related_elements() {
        let answer =
            json!({ "findings": "F", "analysis": "A", "assessment": "S", "verdict": "inconclusive" }).to_string();
        let service = FakeService::start(FakeAnswer::Stream(completions_events(&[&answer]))).await;
        let test = ready(&service).await;
        let session = test.state.session().unwrap();
        {
            let db = session.db();
            let catalog = crate::test_support::db::basic_catalog();
            let mut package = crate::test_support::db::lab_package(catalog.uuid());
            package.package_uuid = "w2".to_owned();
            package.tasks[0].related = vec![Related {
                catalog_uuid: catalog.uuid().to_owned(),
                control_id: "s2.1.1".to_owned(),
                label: "2.1.1".to_owned(),
                title: "Access control policy".to_owned(),
            }];
            let mut evaluation = crate::test_support::db::lab_evaluation(&package);
            evaluation.uuid = "e2".to_owned();
            db_evaluation::create::evaluation(&db, &evaluation, &package).unwrap();
        }
        let (_, channel) = collector();

        propose(&session, "e2", "t", &test.state.start_work(), &channel).await.unwrap();

        let asked = service.received()[0].body.to_string();
        assert!(asked.contains("Within: Organization of Information Security > Internal Organization"));
        assert!(asked.contains("- s1.1.1-prm1: assignment of a choice from a selection; selection of one or more of"));
        assert!(asked.contains("The related elements of the requirement"));
        assert!(asked.contains("2.1.1: "), "the related element word for word from its catalog");
    }

    #[tokio::test]
    async fn the_agent_of_the_evaluation_gives_its_ticked_passages_as_context() {
        let answer = json!({ "findings": "F", "analysis": "A", "assessment": "S", "verdict": "met" }).to_string();
        let service = FakeService::start(FakeAnswer::Stream(completions_events(&[&answer, &answer]))).await;
        let test = ready(&service).await;
        let session = test.state.session().unwrap();
        let (kept, dropped) = {
            let db = session.db();
            let base = crate::db::knowledge::create::base(&db, "ISO", "").unwrap();
            let passages = [
                crate::model_old::knowledge::Passage {
                    page: Some(7),
                    label: String::new(),
                    text: "Information security roles shall be defined and allocated.".to_owned(),
                },
                crate::model_old::knowledge::Passage {
                    page: Some(9),
                    label: String::new(),
                    text: "Responsibilities for information security roles are documented.".to_owned(),
                },
            ];
            crate::db::knowledge::create::source(
                &db,
                base,
                crate::model_old::knowledge::SourceKind::Pdf,
                "ISO 27002",
                &passages,
            )
            .unwrap();
            let itself = crate::model_old::knowledge::Passage {
                page: None,
                label: "1.1.1".to_owned(),
                text: "1.1.1: Information security roles and responsibilities".to_owned(),
            };
            let catalog = "Sample Security Catalog *for Demonstration* and Testing";
            let kind = crate::model_old::knowledge::SourceKind::Catalog;
            crate::db::knowledge::create::source(&db, base, kind, catalog, &[itself]).unwrap();
            let mut fields = crate::views::knowledge::agent_form::view::new_agent("ISO agent");
            fields.bases = vec![base];
            let agent = crate::db::knowledge::create::agent(&db, &fields).unwrap();
            db_evaluation::update::agent(&db, "e", Some(agent)).unwrap();

            let task = lab_task::task(&db, "e", "t").unwrap();
            let found = context::context(&db, "e", &task).unwrap();
            assert_eq!(found.passages.len(), 2, "the requirement itself, from its catalog, is left out");
            let dropped = found.passages[1].0.id;
            db_evaluation::update::context_excluded(&db, "e", "t", dropped, true).unwrap();
            (found.passages[0].0.passage.text.clone(), found.passages[1].0.passage.text.clone())
        };
        let (_, channel) = collector();

        propose(&session, "e", "t", &test.state.start_work(), &channel).await.unwrap();

        let asked = service.received()[0].body.to_string();
        assert!(asked.contains("The context of the standard, from the knowledge of an agent"));
        assert!(asked.contains("ISO agent") && asked.contains(&kept));
        assert!(!asked.contains(&dropped), "an unticked passage stays out");

        db_evaluation::update::task_agent(&session.db(), "e", "t", crate::model_old::evaluation::TaskAgent::NoAgent)
            .unwrap();
        propose(&session, "e", "t", &test.state.start_work(), &channel).await.unwrap();
        assert!(
            !service.received()[1].body.to_string().contains("The context of the standard"),
            "no agent for this task"
        );
    }
}
