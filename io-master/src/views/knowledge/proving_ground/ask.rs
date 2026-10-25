//! The proving ground of an agent (step 57.3 of docs/todo.md): one question
//! of the evaluator answered by the agent through `ai/api/` (task
//! `knowledge_answer`, role *consultation*, in the journal of the AI): the
//! passages of its bases found (`components/knowledge_agent/find.rs`), the
//! answer read, its quotes looked for in the passages they name (P5), and
//! the whole kept as the last trial of the agent, which the page then reads.

use std::sync::Arc;

use tauri::ipc::Channel;
use tokio::sync::watch;

use crate::ai::api::roles::{self, Model};
use crate::ai::api::tasks;
use crate::ai::tasks::knowledge_answer;
use crate::app::error::AppError;
use crate::components::ai_work::ask::{self, Asked};
use crate::components::ai_work::view::AiEvent;
use crate::components::knowledge_agent::find;
use crate::db::knowledge as db_knowledge;
use crate::db::user::database::UserDb;
use crate::installation::session::Session;
use crate::model::ai::Task;
use crate::model::evaluation::quote_found;
use crate::model::knowledge::{Hit, Trial, TrialQuote};

/// What a question is asked with: the model, its text, and the passages in it.
struct Question {
    model: Arc<Model>,
    text: String,
    passages: Vec<Hit>,
}

/// Asks agent `agent` the question `question` and keeps the trial. The
/// events go to `events`; on an answer kept, `Proposed` with one answer
/// added and the quotes not found as `dropped`.
///
/// Called by: `proving_ground_ask` (`knowledge.rs`).
/// Calls: [`question_of`], `ask::ask`, [`store`].
pub async fn ask(
    session: &Session,
    agent: i64,
    question: &str,
    stop: &watch::Receiver<bool>,
    events: &Channel<AiEvent>,
) -> Result<(), AppError> {
    let asked = question_of(&session.db(), agent, question)?;

    let stopped = match ask::ask(session, &asked.model, Task::KnowledgeAnswer, asked.text, stop, events).await {
        Asked::Answer(answer) => {
            let event = store(&session.db(), agent, question, asked.passages, &answer)?;

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

/// The question of agent `agent`: its passages for `question`, its
/// instructions, the rules of the task, and the model.
///
/// Called by: [`ask`].
/// Calls: `find::agent`, `find::passages`, `tasks::instructions`, `knowledge_answer::prompt`, `roles::model`.
fn question_of(db: &UserDb, agent: i64, question: &str) -> Result<Question, AppError> {
    let found = find::agent(db, agent)?;
    let passages = find::passages(db, &found, question)?;
    let rules = tasks::instructions(db, Task::KnowledgeAnswer)?.unwrap_or_default();
    let text = knowledge_answer::prompt(&found.fields.instructions, &rules, &passages, question);

    Ok(Question { model: Arc::new(roles::model(db)?), text, passages })
}

/// Reads `answer`, looks for every quote in the passage it names, and keeps
/// the trial; gives the event of what was kept, or why the answer could
/// not be read.
///
/// Called by: [`ask`].
/// Calls: `knowledge_answer::read`, `quote_found`, `db_knowledge::update::trial`.
fn store(db: &UserDb, agent: i64, question: &str, passages: Vec<Hit>, answer: &str) -> Result<AiEvent, AppError> {
    let (text, read) = match knowledge_answer::read(answer) {
        Ok(read) => read,
        Err(message) => return Ok(AiEvent::Failed { message }),
    };
    let mut quotes = Vec::with_capacity(read.len());
    let mut not_found = Vec::new();

    for (passage, quote) in read {
        let found = found_in(&passages, passage, &quote);

        if !found {
            not_found.push(format!("not found in passage [{passage}]: {quote}"));
        }

        quotes.push(TrialQuote { passage, quote, found });
    }

    let trial = Trial { question: question.trim().to_owned(), passages, answer: text, quotes };
    db_knowledge::update::trial(db, agent, &trial)?;

    Ok(AiEvent::Proposed { added: 1, corrections: 0, dropped: not_found })
}

/// Whether `quote` stands in passage number `passage` (from 1) of
/// `passages` (white space aside, `quote_found`); a number of no passage
/// finds nothing.
///
/// Called by: [`store`].
fn found_in(passages: &[Hit], passage: usize, quote: &str) -> bool {
    let Some(index) = passage.checked_sub(1) else {
        return false;
    };
    let Some(hit) = passages.get(index) else {
        return false;
    };

    quote_found(std::slice::from_ref(&hit.passage.text), None, quote)
}

#[cfg(test)]
mod tests {
    //! Tests of `ask.rs`: the AI is a fake service of the tests.

    use super::*;

    use serde_json::json;

    use crate::ai::api::connections;
    use crate::ai::api::test_support::{FakeAnswer, FakeService, completions, completions_events};
    use crate::app::test_support::logged_in;
    use crate::model::api_key::ApiKey;
    use crate::model::knowledge::{Passage, SourceKind};

    /// A channel that collects the events it gets, as JSON.
    fn collector() -> (std::sync::Arc<std::sync::Mutex<Vec<serde_json::Value>>>, Channel<AiEvent>) {
        let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink = std::sync::Arc::clone(&events);
        let channel = Channel::new(move |body: tauri::ipc::InvokeResponseBody| {
            sink.lock().unwrap().push(body.deserialize::<serde_json::Value>().unwrap());
            Ok(())
        });

        (events, channel)
    }

    #[tokio::test]
    async fn the_agent_answers_from_its_passages_and_its_quotes_are_checked() {
        let answer = json!({ "answer": "Keys are generated by the TSF.", "quotes": [
            { "passage": 1, "quote": "The TSF shall generate cryptographic keys" },
            { "passage": 1, "quote": "The TSF shall invent keys" }
        ] })
        .to_string();
        let service = FakeService::start(FakeAnswer::Stream(completions_events(&[&answer]))).await;
        let test = logged_in().await;
        let session = test.state.session().unwrap();
        let agent = {
            let db = session.db();
            let id = connections::create(&db, &completions(&service)).unwrap().id;
            connections::set_key(&db, id, &ApiKey::new("sk-test".to_owned()).unwrap()).unwrap();
            let base = db_knowledge::create::base(&db, "CC", "").unwrap();
            let text = "The TSF shall generate cryptographic keys in accordance with a specified algorithm.";
            let passage = Passage { page: Some(40), label: "FCS_CKM.1".to_owned(), text: text.to_owned() };
            db_knowledge::create::source(&db, base, SourceKind::Catalog, "CC Part 2", &[passage]).unwrap();
            let mut fields = crate::views::knowledge::agent_form::view::new_agent("CC agent");
            fields.bases = vec![base];
            db_knowledge::create::agent(&db, &fields).unwrap()
        };
        let (events, channel) = collector();

        ask(&session, agent, "How are keys generated?", &test.state.start_work(), &channel).await.unwrap();

        let trial = db_knowledge::read::trial(&session.db(), agent).unwrap().unwrap();
        assert_eq!((trial.answer.as_str(), trial.passages.len()), ("Keys are generated by the TSF.", 1));
        assert!(trial.quotes[0].found && !trial.quotes[1].found, "an invented quote is marked");
        assert_eq!(crate::db::ai::journal::read::last(&session.db(), 5).unwrap()[0].task, "knowledge_answer");
        assert!(service.received()[0].body.to_string().contains("[1] CC Part 2 · FCS_CKM.1 · page 40"));
        assert_eq!(events.lock().unwrap().last().unwrap(), &json!({ "event": "end", "stopped": false }));
    }
}
