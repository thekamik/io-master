//! The chat of an input (step 52 of docs/history.md): the evaluator asks the
//! AI whether the evidence of the input is ready to be assessed. The
//! question goes through `ai/api/` into the journal of the AI, with the
//! instructions of the plan, the requirement and the start of the evidence
//! (`ai/tasks/input_chat.rs`); the question and the answer are kept.

use std::sync::Arc;

use tauri::ipc::Channel;
use tokio::sync::watch;

use crate::ai::api::roles;
use crate::ai::api::tasks;
use crate::ai::tasks::input_chat;
use crate::app::error::AppError;
use crate::components::ai_work::ask::{self, Asked};
use crate::components::ai_work::view::AiEvent;
use crate::db::evaluation as db_evaluation;
use crate::installation::session::Session;
use crate::model::ai::{Speaker, Task, Turn};
use crate::model::evaluation::check_message;
use crate::model::task_scheme::BlockKind;

use super::super::lab_task::read as lab_task;

/// Sends `question` to the chat of input `block` and keeps the question
/// and the answer; gives the task of the input. The events go to `events`;
/// a question stopped or failed keeps nothing.
///
/// Called by: `input_chat_send` (`evaluation_lab.rs`).
/// Calls: `check_message`, [`turns`], `ask::ask`, `db_evaluation::create::message`.
pub async fn send(
    session: &Session,
    evaluation: &str,
    block: &str,
    question: &str,
    stop: &watch::Receiver<bool>,
    events: &Channel<AiEvent>,
) -> Result<String, AppError> {
    check_message(question).map_err(|error| AppError::Invalid(error.to_string()))?;

    let (task, model, turns) = {
        let db = session.db();
        let (task, turns) = turns(&db, evaluation, block, question)?;

        (task, Arc::new(roles::model(&db)?), turns)
    };

    let answer = ask::ask_turns(session, &model, Task::InputChat, &turns, stop, events).await;
    let stopped = !matches!(answer, Asked::Answer(_));

    if let Asked::Answer(answer) = answer {
        let db = session.db();

        db_evaluation::create::message(&db, evaluation, block, Speaker::Evaluator, question.trim())?;
        db_evaluation::create::message(&db, evaluation, block, Speaker::Model, answer.trim())?;
        db_evaluation::update::task_started(&db, evaluation, &task)?;
    }

    ensure!(
        events.send(AiEvent::End { stopped }).is_ok(),
        AppError::Failed("the window can no longer be reached".to_owned())
    );

    Ok(task)
}

/// The conversation for `question`, and the task of input `block`.
///
/// Called by: [`send`].
/// Calls: `db_evaluation::read::block`, `lab_task::task` and `requirement`,
/// `read::evidence_of`, `read::messages`, `tasks::instructions`, `input_chat::turns`.
fn turns(
    db: &crate::db::user::database::UserDb,
    evaluation: &str,
    block: &str,
    question: &str,
) -> Result<(String, Vec<Turn>), AppError> {
    let Some((task, input)) = db_evaluation::read::block(db, evaluation, block)? else {
        return Err(AppError::Invalid("the input does not exist".to_owned()));
    };

    ensure!(input.kind == BlockKind::Input, AppError::Invalid("only an input has a chat".to_owned()));

    let (_, requirement, _) = lab_task::requirement(db, &lab_task::task(db, evaluation, &task)?)?;
    let evidence = db_evaluation::read::evidence_of(db, evaluation, block)?;
    let history = db_evaluation::read::messages(db, evaluation, block)?;
    let instructions = tasks::instructions(db, Task::InputChat)?.unwrap_or_default();

    Ok((task, input_chat::turns(&instructions, &requirement, &input.fields, &evidence, &history, question.trim())))
}

#[cfg(test)]
mod tests {
    //! Tests of `chat.rs`: the AI is a fake service of the tests.

    use super::*;

    use crate::ai::api::test_support::{FakeAnswer, FakeService, completions_events};
    use crate::views::evaluation_lab::test_support::{NOTE, collector, ready};

    #[tokio::test]
    async fn the_question_and_the_answer_are_kept_and_the_evidence_is_shown_to_the_model() {
        let service =
            FakeService::start(FakeAnswer::Stream(completions_events(&["It names an owner for the keys."]))).await;
        let test = ready(&service).await;
        let session = test.state.session().unwrap();
        let (_, channel) = collector();

        assert_eq!(send(&session, "e", "i", " Is it enough? ", &test.state.start_work(), &channel).await.unwrap(), "t");

        let messages = db_evaluation::read::messages(&session.db(), "e", "i").unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(
            (messages[0].text.as_str(), messages[1].text.as_str()),
            ("Is it enough?", "It names an owner for the keys.")
        );
        assert!(service.received()[0].body.to_string().contains(&NOTE[..20]));
        assert!(send(&session, "e", "i", " ", &test.state.start_work(), &channel).await.is_err());
        assert!(send(&session, "e", "o", "Hi", &test.state.start_work(), &channel).await.is_err());
    }
}
