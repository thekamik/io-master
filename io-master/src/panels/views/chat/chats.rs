//! The chats, as the page calls them (user, 2026-10-30): the list, a new
//! chat with an agent, one read and renamed, a question whose answer is
//! streamed into the page, and "Stop".
//!
//! ```text
//! ChatList.svelte ──chat_list()───────────────► read_chats ──► [ChatSummaryView]
//!                 ──chat_create(title, agent)─► new_chat ──► id
//! Chat.svelte     ──chat_get(id)──────────────► read_chat ──► ChatView
//!                 ──chat_rename(id, title)────► rename_chat
//!                 ──chat_ask(id, text, on_piece):
//!                     (the lock of the session held)  chat_question, ask::prepare, add_question
//!                     (released)                      ask::send ──pieces──► forwarded ──on_piece──► the page
//!                     (held)                          ask::record (the journal of the agent), add_answer
//!                     ──► ChatAnswerView
//!                 ──chat_stop()───────────────► the session asked to stop: the answer ends at once
//! ```
//!
//! A question is a long work of the AI: only one runs at a time, a chat or
//! "Fill with AI" (`Session::begin_work`).

use std::time::Duration;

use serde::Serialize;
use tauri::State;
use tauri::ipc::Channel;
use tokio::sync::mpsc;

use crate::ai::ask::{Prepared, Sent, prepare, record, send};
use crate::db::ai::agent::read_agent;
use crate::db::ai::chat::{read_chat, read_chats};
use crate::db::database::Database;
use crate::model::ai::chat::{Chat, Message, Speaker};
use crate::model::ai::journal::Ending;
use crate::panels::logic::chat::chats::{MAX_QUESTION, add_answer, add_question, chat_question, new_chat, rename_chat};
use crate::panels::views::error::ViewError;
use crate::panels::views::opened::database;
use crate::session::state::Session;

/// The longest title from the page, in bytes (the logic checks the
/// characters).
const MAX_TITLE_BYTES: usize = 4096;

/// How often the forwarding of an answer looks whether the user asked it
/// to stop.
const STOP_CHECK: Duration = Duration::from_millis(200);

/// The most steps of the forwarding of an answer: a piece or a look for a
/// stop. Every piece has at least one byte of the stream, whose limit is 32
/// MiB (`ai/client.rs`); the looks are as many again, about 77 days of an
/// answer, which the silence of the stream (5 minutes) ends long before.
const MAX_STEPS: usize = 2 * 32 * 1024 * 1024;

/// A chat, as the list shows it.
#[derive(Serialize)]
pub struct ChatSummaryView {
    id: i64,
    title: String,
    /// The name of its agent.
    agent: String,
    messages: usize,
}

/// A message of a chat.
#[derive(Serialize)]
pub struct MessageView {
    /// "user" or "ai".
    speaker: &'static str,
    text: String,
    /// Seconds since 1970 (UTC).
    time: i64,
}

/// A chat whole.
#[derive(Serialize)]
pub struct ChatView {
    id: i64,
    title: String,
    agent: String,
    messages: Vec<MessageView>,
}

/// How a question of a chat ended.
#[derive(Serialize)]
pub struct ChatAnswerView {
    /// The answer as the chat keeps it; `None` when it failed or was empty.
    answer: Option<MessageView>,
    /// "complete", "cut", "stopped", "refused", "failed".
    ending: &'static str,
    /// Why it failed.
    error: Option<String>,
}

/// The view of the message `message`.
///
/// Called by: [`chat_view`], [`chat_ask`].
fn message_view(message: Message) -> MessageView {
    let speaker = match message.speaker {
        Speaker::User => "user",
        Speaker::Ai => "ai",
    };

    MessageView { speaker, text: message.text, time: message.time }
}

/// The name of the agent of a chat; "(no agent)" when it names none.
///
/// Called by: [`chat_list`], [`chat_view`].
/// Calls: `read_agent`.
fn agent_name(db: &Database, agent: Option<i64>) -> Result<String, ViewError> {
    Ok(match agent {
        Some(id) => read_agent(db, id)?.name,
        None => "(no agent)".to_owned(),
    })
}

/// The view of the chat `chat`.
///
/// Called by: [`chat_get`].
/// Calls: [`agent_name`], [`message_view`].
fn chat_view(db: &Database, chat: Chat) -> Result<ChatView, ViewError> {
    let agent = agent_name(db, chat.agent)?;
    let mut messages = Vec::with_capacity(chat.messages.len());

    for message in chat.messages {
        messages.push(message_view(message));
    }

    Ok(ChatView { id: chat.id, title: chat.title, agent, messages })
}

/// The name of an ending on the page.
///
/// Called by: [`chat_ask`].
fn ending_name(ending: Ending) -> &'static str {
    match ending {
        Ending::Complete => "complete",
        Ending::Cut => "cut",
        Ending::Stopped => "stopped",
        Ending::Refused => "refused",
        Ending::Failed => "failed",
    }
}

/// Every chat, the newest first.
///
/// Called by: `ChatList.svelte`, when the panel is opened and after a change.
/// Calls: `read_chats`, [`agent_name`].
#[tauri::command]
pub fn chat_list(session: State<'_, Session>) -> Result<Vec<ChatSummaryView>, ViewError> {
    let opened = session.lock()?;
    let db = database(&opened)?;
    let chats = read_chats(db)?;
    let mut views = Vec::with_capacity(chats.len());

    for chat in chats {
        let agent = agent_name(db, chat.agent)?;

        views.push(ChatSummaryView { id: chat.id, title: chat.title, agent, messages: chat.messages.len() });
    }

    Ok(views)
}

/// Makes a new chat titled `title` with the agent `agent`; gives its id.
///
/// Called by: `ChatList.svelte`, "New chat".
/// Calls: `new_chat`.
#[tauri::command]
pub fn chat_create(session: State<'_, Session>, title: String, agent: i64) -> Result<i64, ViewError> {
    ensure!(title.len() <= MAX_TITLE_BYTES, ViewError::Invalid("the title is too long".to_owned()));

    let opened = session.lock()?;

    Ok(new_chat(database(&opened)?, &title, agent)?)
}

/// The chat `id` whole.
///
/// Called by: `Chat.svelte`, when a chat is opened.
/// Calls: `read_chat`, [`chat_view`].
#[tauri::command]
pub fn chat_get(session: State<'_, Session>, id: i64) -> Result<ChatView, ViewError> {
    let opened = session.lock()?;
    let db = database(&opened)?;

    chat_view(db, read_chat(db, id)?)
}

/// Gives the chat `id` the title `title`.
///
/// Called by: `Chat.svelte`, "Rename".
/// Calls: `rename_chat`.
#[tauri::command]
pub fn chat_rename(session: State<'_, Session>, id: i64, title: String) -> Result<(), ViewError> {
    ensure!(title.len() <= MAX_TITLE_BYTES, ViewError::Invalid("the title is too long".to_owned()));

    let opened = session.lock()?;

    Ok(rename_chat(database(&opened)?, id, &title)?)
}

/// The question `text` in the chat `chat`, prepared, and written into the
/// chat once it was taken.
///
/// Called by: [`chat_ask`].
/// Calls: `chat_question`, `ask::prepare`, `add_question`.
fn prepared_question(session: &Session, chat: i64, text: &str) -> Result<Prepared, ViewError> {
    let opened = session.lock()?;
    let db = database(&opened)?;
    let prepared = prepare(db, &chat_question(db, chat, text)?)?;

    add_question(db, chat, text)?;

    Ok(prepared)
}

/// Sends the prepared question and passes each piece of its answer to the
/// page through `on_piece`, while it comes. The answer is stopped (its
/// receiver dropped) when the user asks it to stop or the page no more
/// listens. Uses no database.
///
/// Called by: [`chat_ask`].
/// Calls: `ask::send`, `Session::stop_requested`.
async fn forwarded(session: &Session, prepared: &Prepared, on_piece: &Channel<String>) -> Sent {
    let (pieces, mut receiver) = mpsc::channel::<String>(64);
    let sending = async move { send(prepared, &pieces).await };
    let forwarding = async move {
        let mut stop_check = tokio::time::interval(STOP_CHECK);

        for _ in 0..MAX_STEPS {
            let go_on = tokio::select! {
                piece = receiver.recv() => match piece {
                    Some(piece) => on_piece.send(piece).is_ok(),
                    None => false,
                },
                _ = stop_check.tick() => !session.stop_requested(),
            };

            if !go_on {
                break;
            }
        }
    };
    let (sent, ()) = tokio::join!(sending, forwarding);

    sent
}

/// Writes the answer into the journal of the agent, then into the chat
/// `chat`; gives what the page shows of it.
///
/// Called by: [`chat_ask`].
/// Calls: `ask::record`, `add_answer`, [`message_view`], [`ending_name`].
fn written(session: &Session, chat: i64, prepared: Prepared, sent: Sent) -> Result<ChatAnswerView, ViewError> {
    let opened = session.lock()?;
    let db = database(&opened)?;
    let answer = record(db, prepared, sent)?;
    let kept = add_answer(db, chat, &answer)?;

    Ok(ChatAnswerView {
        answer: kept.map(message_view),
        ending: ending_name(answer.ending),
        error: answer.error.map(|error| error.to_string()),
    })
}

/// Asks the question `text` in the chat `chat`: the answer is passed to the
/// page piece by piece through `on_piece`, and kept in the chat and in the
/// journal of its agent. Refused while another work of the AI runs.
///
/// Called by: `Chat.svelte`, "Send".
/// Calls: `Session::begin_work`, [`prepared_question`], [`forwarded`],
/// [`written`].
#[tauri::command]
pub async fn chat_ask(
    session: State<'_, Session>,
    chat: i64,
    text: String,
    on_piece: Channel<String>,
) -> Result<ChatAnswerView, ViewError> {
    ensure!(text.len() <= 4 * MAX_QUESTION, ViewError::Invalid("the question is too long".to_owned()));

    let Some(_work) = session.begin_work() else {
        return Err(ViewError::Invalid("another work of the AI runs: wait for it or stop it".to_owned()));
    };
    let prepared = prepared_question(&session, chat, &text)?;
    let sent = forwarded(&session, &prepared, &on_piece).await;

    written(&session, chat, prepared, sent)
}

/// Asks the answer that is being written to stop at once.
///
/// Called by: `Chat.svelte`, "Stop".
/// Calls: `Session::request_stop`.
#[tauri::command]
pub fn chat_stop(session: State<'_, Session>) {
    session.request_stop();
}

#[cfg(test)]
mod tests {
    //! Tests of the forwarding of an answer: every piece reaches the page,
    //! and "Stop" ends an answer that is still being written.

    use super::*;

    use std::sync::{Arc, Mutex};

    use serde_json::json;
    use tauri::ipc::InvokeResponseBody;

    use crate::ai::ask::Question;
    use crate::db::ai::agent::create_agent;
    use crate::db::ai::integration::create_integration;
    use crate::model::ai::integration::Integration;
    use crate::test_support::TempDir;
    use crate::test_support::ai_service::{FakeAnswer, FakeService, openai_events, sse};
    use crate::test_support::database::{agent, database, integration};

    /// The question "Hi" of an agent to the fake service `service`.
    fn prepared(db: &Database, service: &FakeService) -> Prepared {
        let local = Integration {
            url: format!("{}/v1/chat/completions", service.url),
            ca_certificate: service.ca_pem.clone(),
            ..integration()
        };
        let id = create_integration(db, &local).unwrap();
        let question = Question {
            integration: id,
            agent: Some(create_agent(db, &agent(id)).unwrap()),
            work_unit: None,
            system: "Be brief.".to_owned(),
            messages: vec![Message { speaker: Speaker::User, text: "Hi".to_owned(), time: 0 }],
        };

        prepare(db, &question).unwrap()
    }

    /// A channel that keeps what the page would get.
    fn page() -> (Channel<String>, Arc<Mutex<Vec<String>>>) {
        let got = Arc::new(Mutex::new(Vec::new()));
        let kept = Arc::clone(&got);
        let channel = Channel::new(move |body| {
            if let InvokeResponseBody::Json(piece) = body {
                kept.lock().unwrap().push(serde_json::from_str(&piece).unwrap());
            }
            Ok(())
        });

        (channel, got)
    }

    #[tokio::test]
    async fn every_piece_of_the_answer_reaches_the_page() {
        let service = FakeService::start(FakeAnswer::Stream(openai_events(&["PA", "SS"]))).await;
        let (_directory, db) = database();
        let question = prepared(&db, &service);
        let folder = TempDir::new();
        let session = Session::new(folder.path().to_path_buf());
        let (channel, got) = page();
        let sent = forwarded(&session, &question, &channel).await;
        let answer = record(&db, question, sent).unwrap();

        assert_eq!((answer.text.as_str(), answer.ending), ("PASS", Ending::Complete));
        assert_eq!(*got.lock().unwrap(), ["PA", "SS"]);
    }

    #[tokio::test]
    async fn stop_ends_an_answer_that_is_still_being_written() {
        let piece = sse("", &json!({"model": "local-model", "choices": [{"delta": {"content": "Hal"}}]}));
        let service = FakeService::start(FakeAnswer::StreamThenWait(vec![piece])).await;
        let (_directory, db) = database();
        let question = prepared(&db, &service);
        let folder = TempDir::new();
        let session = Session::new(folder.path().to_path_buf());
        let (channel, got) = page();
        let stopping = async {
            tokio::time::sleep(Duration::from_millis(500)).await;
            session.request_stop();
        };
        let forwarding = tokio::time::timeout(Duration::from_secs(10), forwarded(&session, &question, &channel));
        let (sent, ()) = tokio::join!(forwarding, stopping);
        let answer = record(&db, question, sent.unwrap()).unwrap();

        assert_eq!((answer.text.as_str(), answer.ending), ("Hal", Ending::Stopped));
        assert_eq!(*got.lock().unwrap(), ["Hal"]);
    }
}
