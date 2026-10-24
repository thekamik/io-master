//! The AI proposes the fragments of the evidence of an input (step 52 of
//! docs/history.md): every piece of its evidence, part by part
//! (`ai/tasks/evidence_text.rs`), one question of the task
//! `evidence_fragments` per part through `ai/api/`. The program looks for
//! every quote in the evidence (`quote_found`) and keeps it with the answer;
//! the evaluator accepts the fragments. The proposals left undecided are
//! removed first.

use std::sync::Arc;

use tauri::ipc::Channel;
use tokio::sync::watch;

use crate::ai::api::roles::{self, Model};
use crate::ai::api::tasks;
use crate::ai::tasks::evidence_fragments;
use crate::ai::tasks::evidence_text::{self, Part};
use crate::app::error::AppError;
use crate::components::ai_work::ask::{self, Asked};
use crate::components::ai_work::view::AiEvent;
use crate::db::evaluation as db_evaluation;
use crate::db::oscal::ids::new_uuid;
use crate::db::user::database::UserDb;
use crate::installation::session::Session;
use crate::model::ai::Task;
use crate::model::evaluation::{Evidence, Fragment, MAX_FRAGMENTS, check_fragment, quote_found};
use crate::model::task_scheme::{BlockFields, BlockKind};

use super::super::lab_task::read as lab_task;

/// What the questions need: the model, the instructions, the requirement as
/// a question shows it, the input and its evidence.
struct Question {
    model: Arc<Model>,
    instructions: String,
    task: String,
    requirement: String,
    input: BlockFields,
    evidence: Vec<Evidence>,
}

/// Asks the AI for the fragments of the evidence of input `block`, piece by
/// piece and part by part, and stores them; gives the task of the input.
/// The events go to `events`; `stop` ends the work after the question that
/// runs.
///
/// Called by: `fragments_propose` (`evaluation_lab.rs`).
/// Calls: [`question`], [`ask_part`].
pub async fn propose(
    session: &Session,
    evaluation: &str,
    block: &str,
    stop: &watch::Receiver<bool>,
    events: &Channel<AiEvent>,
) -> Result<String, AppError> {
    let question = question(&session.db(), evaluation, block)?;
    let mut parts = Vec::new();

    for piece in &question.evidence {
        for part in evidence_text::parts(piece) {
            parts.push((piece, part));
        }
    }

    let mut stopped = false;

    for (index, (piece, part)) in parts.iter().enumerate() {
        if *stop.borrow() || events.send(AiEvent::Part { index: index + 1, count: parts.len() }).is_err() {
            stopped = true;
            break;
        }

        if !ask_part(session, evaluation, block, &question, piece, *part, stop, events).await? {
            stopped = true;
            break;
        }
    }

    ensure!(
        events.send(AiEvent::End { stopped }).is_ok(),
        AppError::Failed("the window can no longer be reached".to_owned())
    );

    Ok(question.task)
}

/// Removes the fragments of input `block` the AI proposed and the evaluator
/// did not accept, and gathers the question; refused for a block that is no
/// input or has no evidence.
///
/// Called by: [`propose`].
fn question(db: &UserDb, evaluation: &str, block: &str) -> Result<Question, AppError> {
    let Some((task, input)) = db_evaluation::read::block(db, evaluation, block)? else {
        return Err(AppError::Invalid("the input does not exist".to_owned()));
    };

    ensure!(input.kind == BlockKind::Input, AppError::Invalid("only an input has fragments".to_owned()));

    let evidence = db_evaluation::read::evidence_of(db, evaluation, block)?;
    ensure!(!evidence.is_empty(), AppError::Invalid("the input has no evidence yet".to_owned()));

    let (_, requirement, _) = lab_task::requirement(db, &lab_task::task(db, evaluation, &task)?)?;
    db_evaluation::delete::proposed_fragments(db, evaluation, block)?;

    Ok(Question {
        model: Arc::new(roles::model(db)?),
        instructions: tasks::instructions(db, Task::EvidenceFragments)?.unwrap_or_default(),
        task,
        requirement,
        input: input.fields,
        evidence,
    })
}

/// Asks about `part` of `piece` and stores the fragments of the answer;
/// gives `false` when the work is stopped or the page is gone.
///
/// Called by: [`propose`].
/// Calls: `evidence_fragments::prompt`, `ask::ask`, [`store`].
// One question needs all of them; a struct would only rename them.
#[allow(clippy::too_many_arguments)]
async fn ask_part(
    session: &Session,
    evaluation: &str,
    block: &str,
    question: &Question,
    piece: &Evidence,
    part: Part,
    stop: &watch::Receiver<bool>,
    events: &Channel<AiEvent>,
) -> Result<bool, AppError> {
    let pages = evidence_text::pages_text(piece, part);
    let text =
        evidence_fragments::prompt(&question.instructions, &question.requirement, &question.input, &piece.name, &pages);

    match ask::ask(session, &question.model, Task::EvidenceFragments, text, stop, events).await {
        Asked::Answer(answer) => {
            let event = store(&session.db(), evaluation, &question.task, block, piece, part, &answer)?;

            Ok(events.send(event).is_ok())
        }
        Asked::Failed => Ok(true),
        Asked::Stopped => Ok(false),
    }
}

/// Reads `answer` for `part` of `piece` and stores its fragments, each with
/// whether its quote is found on its page of the part (else anywhere in the
/// part); a page outside the part counts as not given. Gives the event of
/// what was stored, or why the answer could not be read.
///
/// Called by: [`ask_part`].
/// Calls: `evidence_fragments::read`, `check_fragment`, `quote_found`,
/// `db_evaluation::create::fragment`.
fn store(
    db: &UserDb,
    evaluation: &str,
    task: &str,
    block: &str,
    piece: &Evidence,
    part: Part,
    answer: &str,
) -> Result<AiEvent, AppError> {
    let (proposed, mut dropped) = match evidence_fragments::read(answer, MAX_FRAGMENTS) {
        Ok(read) => read,
        Err(message) => return Ok(AiEvent::Failed { message }),
    };
    let mut added = 0;

    for fragment in proposed {
        if let Err(error) = check_fragment(&fragment.quote, &fragment.note) {
            dropped.push(error.to_string());
            continue;
        }

        let page = fragment
            .page
            .filter(|page| usize::try_from(*page).is_ok_and(|number| number >= part.first && number <= part.last));
        let found = quote_found(&piece.pages, page, &fragment.quote);
        let kept = Fragment {
            uuid: new_uuid()?,
            evidence: piece.uuid.clone(),
            page,
            quote: fragment.quote,
            note: fragment.note,
            found,
            accepted: false,
            by_ai: true,
        };

        if let Err(error) = db_evaluation::create::fragment(db, evaluation, task, block, &kept) {
            dropped.push(error.to_string());
            break;
        }

        added += 1;
    }

    Ok(AiEvent::Proposed { added, corrections: 0, dropped })
}

#[cfg(test)]
mod tests {
    //! Tests of `propose.rs`: the AI is a fake service of the tests.

    use super::*;

    use serde_json::json;

    use crate::ai::api::test_support::{FakeAnswer, FakeService, completions_events};
    use crate::views::evaluation_lab::test_support::{collector, ready};

    #[tokio::test]
    async fn the_fragments_are_kept_with_whether_their_quote_is_found() {
        let answer = json!({ "fragments": [
            { "page": 1, "quote": "Anna owns the keys\nof the TOE.", "note": "An owner." },
            { "page": 1, "quote": "Carol owns nothing.", "note": "" }
        ] })
        .to_string();
        let service = FakeService::start(FakeAnswer::Stream(completions_events(&[&answer]))).await;
        let test = ready(&service).await;
        let session = test.state.session().unwrap();
        let (events, channel) = collector();

        assert_eq!(propose(&session, "e", "i", &test.state.start_work(), &channel).await.unwrap(), "t");

        let fragments = db_evaluation::read::fragments(&session.db(), "e", "i").unwrap();
        assert_eq!(fragments.len(), 2);
        assert!(fragments[0].found && fragments[0].by_ai && !fragments[0].accepted);
        assert!(!fragments[1].found, "a quote that is not in the evidence");
        assert_eq!(events.lock().unwrap().last().unwrap(), &json!({ "event": "end", "stopped": false }));
        assert_eq!(crate::db::ai::journal::read::last(&session.db(), 5).unwrap()[0].task, "evidence_fragments");
        assert!(service.received()[0].body.to_string().contains("=== Page 1 ==="));
    }

    #[tokio::test]
    async fn an_output_has_no_fragments() {
        let service = FakeService::start(FakeAnswer::Stream(completions_events(&["{}"]))).await;
        let test = ready(&service).await;
        let session = test.state.session().unwrap();
        let (_, channel) = collector();

        assert!(propose(&session, "e", "o", &test.state.start_work(), &channel).await.is_err());
    }
}
