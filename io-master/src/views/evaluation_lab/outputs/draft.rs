//! The AI drafts the text of an output (step 52 of docs/history.md): one
//! question of the task `output_draft` through `ai/api/`, with the four
//! results and the verdict the evaluator decided, as the output asks (its
//! target, format and instructions). Only an output whose condition holds
//! for the verdict is drafted. The draft is kept beside the accepted text;
//! the evaluator accepts or corrects it.

use std::sync::Arc;

use tauri::ipc::Channel;
use tokio::sync::watch;

use crate::ai::api::roles::{self, Model};
use crate::ai::api::tasks;
use crate::ai::tasks::knowledge_context;
use crate::ai::tasks::output_draft;
use crate::app::error::AppError;
use crate::components::ai_work::ask::{self, Asked};
use crate::components::ai_work::view::AiEvent;
use crate::db::evaluation as db_evaluation;
use crate::db::user::database::UserDb;
use crate::installation::session::Session;
use crate::model_old::ai::Task;
use crate::model_old::evaluation::{check_output, output_applies};
use crate::model_old::task_scheme::BlockKind;

use super::super::assessment::context;
use super::super::lab_task::read as lab_task;

/// Asks the AI for the text of output `block` and keeps the draft; gives the
/// task of the output. The events go to `events`.
///
/// Called by: `outputs_draft` (`evaluation_lab.rs`).
/// Calls: [`question`], `ask::ask`, `output_draft::read`, `check_output`,
/// `db_evaluation::update::output_proposal`.
pub async fn draft(
    session: &Session,
    evaluation: &str,
    block: &str,
    stop: &watch::Receiver<bool>,
    events: &Channel<AiEvent>,
) -> Result<String, AppError> {
    let (task, model, text) = question(&session.db(), evaluation, block)?;

    let stopped = match ask::ask(session, &model, Task::OutputDraft, text, stop, events).await {
        Asked::Answer(answer) => {
            let written = output_draft::read(&answer);

            let event = match check_output(&written) {
                Ok(()) => {
                    db_evaluation::update::output_proposal(&session.db(), evaluation, block, &written)?;
                    AiEvent::Proposed { added: 1, corrections: 0, dropped: Vec::new() }
                }
                Err(error) => AiEvent::Failed { message: error.to_string() },
            };

            events.send(event).is_err()
        }
        Asked::Failed => false,
        Asked::Stopped => true,
    };

    ensure!(
        events.send(AiEvent::End { stopped }).is_ok(),
        AppError::Failed("the window can no longer be reached".to_owned())
    );

    Ok(task)
}

/// The task of output `block`, the model and the question; refused for a
/// block that is no output, or whose condition does not hold for the
/// verdict.
///
/// Called by: [`draft`].
/// Calls: `db_evaluation::read::block`, `lab_task::task`, `read::results`,
/// `output_applies`, `output_draft::prompt`, `context::question_text` (the
/// context of the agent of the task, step 57.6), `knowledge_context::appended`.
fn question(db: &UserDb, evaluation: &str, block: &str) -> Result<(String, Arc<Model>, String), AppError> {
    let Some((task, output)) = db_evaluation::read::block(db, evaluation, block)? else {
        return Err(AppError::Invalid("the output does not exist".to_owned()));
    };

    ensure!(output.kind == BlockKind::Output, AppError::Invalid("only an output is drafted".to_owned()));

    let found = lab_task::task(db, evaluation, &task)?;
    let results = db_evaluation::read::results(db, evaluation, &task)?;

    ensure!(
        output_applies(output.fields.condition, results.verdict),
        AppError::Invalid("the condition of the output does not hold for the verdict".to_owned())
    );

    let instructions = tasks::instructions(db, Task::OutputDraft)?.unwrap_or_default();
    let text = knowledge_context::appended(
        output_draft::prompt(&instructions, &output.fields, &found.label, &found.title, &results),
        &context::question_text(db, evaluation, &found)?,
    );

    Ok((task, Arc::new(roles::model(db)?), text))
}

#[cfg(test)]
mod tests {
    //! Tests of `draft.rs`: the AI is a fake service of the tests.

    use super::*;

    use crate::ai::api::test_support::{FakeAnswer, FakeService, completions_events};
    use crate::model_old::evaluation::{Results, Verdict};
    use crate::views::evaluation_lab::test_support::{collector, ready};

    #[tokio::test]
    async fn an_output_is_drafted_only_when_its_condition_holds() {
        let service =
            FakeService::start(FakeAnswer::Stream(completions_events(&["```latex\n\\paragraph{Roles} Met.\n```"])))
                .await;
        let test = ready(&service).await;
        let session = test.state.session().unwrap();
        let mut results = Results::empty();
        results.findings = "Anna owns the keys.".to_owned();
        results.verdict = Some(Verdict::Met);
        db_evaluation::update::results(&session.db(), "e", "t", &results).unwrap();
        let (_, channel) = collector();

        assert_eq!(draft(&session, "e", "o", &test.state.start_work(), &channel).await.unwrap(), "t");
        let output = db_evaluation::read::output(&session.db(), "e", "o").unwrap().unwrap();
        assert_eq!(output.proposal.as_deref(), Some("\\paragraph{Roles} Met."));
        assert_eq!(output.text, "", "the evaluator accepts it");

        assert!(
            draft(&session, "e", "n", &test.state.start_work(), &channel).await.is_err(),
            "not met is not the verdict"
        );
        assert!(service.received()[0].body.to_string().contains("Anna owns the keys."));
    }
}
