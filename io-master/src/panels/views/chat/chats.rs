//! The chats, as the page calls them (user, 2026-10-30): the list, a new
//! chat with an agent, one read and renamed, a question whose answer is
//! streamed into the page, and "Stop".
//!
//! ```text
//! ChatList.svelte     ──chat_list()───────────────► read_chats ──► [ChatSummaryView]
//!                     ──chat_create(title, agent)─► new_chat ──► id
//! Conversation.svelte ──chat_get(id)──────────────► read_chat ──► ChatView
//!                     ──chat_rename(id, title)────► rename_chat
//!                     ──chat_ask(id, text, on_piece):
//!                         (the lock of the session held)  prepared (the oldest messages left out
//!                                                         when they do not fit), add_question
//!                         (released)                      sending::send ──pieces──► forwarded ──on_piece──► the page
//!                         (held)                          journal::record (the journal of the agent), add_answer
//!                         ──► ChatAnswerView
//!                     ──chat_stop()───────────────► the session asked to stop: the answer ends at once
//! ```
//!
//! A question is a long work of the AI: only one runs at a time, a chat or
//! "Fill with AI" (`Session::begin_work`).

use std::time::Duration;

use serde::Serialize;
use tauri::State;
use tauri::ipc::Channel;
use tokio::sync::mpsc;

use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::ai::agent::agent_store::read_agent;
use crate::objects::ai::chat::chat_ai::{add_answer, prepared};
use crate::objects::ai::chat::chat_model::{Chat, Message, Speaker};
use crate::objects::ai::chat::chat_rules::{MAX_QUESTION, add_question, new_chat, rename_chat};
use crate::objects::ai::chat::chat_store::{read_chat, read_chats};
use crate::objects::ai::integration::integration_ai::question::Prepared;
use crate::objects::ai::integration::integration_ai::sending::{Sent, send};
use crate::objects::ai::journal::journal_ai::record;
use crate::objects::ai::journal::journal_model::Ending;
use crate::panels::session::Session;
use crate::panels::views::error::ViewError;
use crate::panels::views::opened::database;

/// The longest title from the page, in bytes (the logic checks the
/// characters).
const MAX_TITLE_BYTES: usize = 4096;

/// How often the forwarding of an answer looks whether the user asked it
/// to stop.
const STOP_CHECK: Duration = Duration::from_millis(200);

/// The most steps of the forwarding of an answer: a piece or a look for a
/// stop. Every piece has at least one byte of the stream, whose limit is 32
/// MiB (`objects/ai/integration/integration_ai/connection.rs`); the looks are as many again, about 77 days of an
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
#[serde(rename_all = "camelCase")]
pub struct ChatAnswerView {
    /// The answer as the chat keeps it; `None` when it failed or was empty.
    answer: Option<MessageView>,
    /// "complete", "cut", "stopped", "refused", "failed".
    ending: &'static str,
    /// Why it failed.
    error: Option<String>,
    /// How many of the oldest messages of the chat were left out of the
    /// question, which did not fit with them.
    left_out: usize,
}

/// The view of the message `message`.
///
/// Input: `message`, a message of a chat.
/// Output: its view (`MessageView { speaker: "user", text: "...", time:
/// 1793577600 }`).
///
/// Called by: [`chat_view`], [`written`].
fn message_view(message: Message) -> MessageView {
    let speaker = match message.speaker {
        Speaker::User => "user",
        Speaker::Ai => "ai",
    };

    MessageView { speaker, text: message.text, time: message.time }
}

/// The name of the agent of a chat; "(no agent)" when it names none.
///
/// Input: `db`, the open database; `agent`, the id of the agent of a chat.
/// Output: its name (`"Evaluator"`), "(no agent)" for none; an error of the
/// database.
///
/// Called by: [`chat_list`], [`chat_view`].
/// Calls: `objects/ai/agent/agent_store.rs`: `read_agent`.
fn agent_name(db: &UserDatabase, agent: Option<i64>) -> Result<String, ViewError> {
    Ok(match agent {
        Some(id) => read_agent(db, id)?.name,
        None => "(no agent)".to_owned(),
    })
}

/// The view of the chat `chat`.
///
/// Input: `db`, the open database; `chat`, a chat with its messages.
/// Output: its view with the name of its agent; an error of the database.
///
/// Called by: [`chat_get`].
/// Calls: [`agent_name`], [`message_view`].
fn chat_view(db: &UserDatabase, chat: Chat) -> Result<ChatView, ViewError> {
    let agent = agent_name(db, chat.agent)?;
    let mut messages = Vec::with_capacity(chat.messages.len());

    for message in chat.messages {
        messages.push(message_view(message));
    }

    Ok(ChatView { id: chat.id, title: chat.title, agent, messages })
}

/// The name of an ending on the page.
///
/// Input: `ending`, how an answer ended.
/// Output: its name on the page (`"stopped"`).
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
/// Input: `session`, the session of the window, someone logged in.
/// Output: the chats (`[{"id": 5, "title": "...", "agent": "Evaluator"}]`);
/// `ViewError::NotLoggedIn` when nobody is logged in, an error of the database.
///
/// Called by: `ui/src/lib/api/chat.ts`: `chatList`, from `ui/src/views/chat/ChatList.svelte`, when the panel is
/// opened and after a change.
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/ai/chat/chat_store.rs`: `read_chats`; [`agent_name`].
#[tauri::command]
pub fn chat_list(session: State<'_, Session>) -> Result<Vec<ChatSummaryView>, ViewError> {
    let opened = session.lock()?;
    let db = database(opened.as_ref())?;
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
/// Input: `session`, the session of the window, someone logged in; `title`, the
/// title typed; `agent`, the id of an agent.
/// Output: the id of the new chat (`5`); `ViewError::NotLoggedIn` when nobody
/// is logged in, the refusals of the chat.
///
/// Called by: `ui/src/lib/api/chat.ts`: `chatCreate`, from `ui/src/views/chat/ChatList.svelte`, "New chat".
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/ai/chat/chat_rules.rs`: `new_chat`.
#[tauri::command]
pub fn chat_create(session: State<'_, Session>, title: String, agent: i64) -> Result<i64, ViewError> {
    ensure!(title.len() <= MAX_TITLE_BYTES, ViewError::Invalid("the title is too long".to_owned()));

    let opened = session.lock()?;

    Ok(new_chat(database(opened.as_ref())?, &title, agent)?)
}

/// The chat `id` whole.
///
/// Input: `session`, the session of the window, someone logged in; `id`, the id
/// of a chat.
/// Output: the chat with its messages; `ViewError::NotLoggedIn` when nobody is
/// logged in, an error of the database.
///
/// Called by: `ui/src/lib/api/chat.ts`: `chatGet`, from `ui/src/views/chat/Conversation.svelte`, when a chat is
/// opened.
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/ai/chat/chat_store.rs`: `read_chat`; [`chat_view`].
#[tauri::command]
pub fn chat_get(session: State<'_, Session>, id: i64) -> Result<ChatView, ViewError> {
    let opened = session.lock()?;
    let db = database(opened.as_ref())?;

    chat_view(db, read_chat(db, id)?)
}

/// Gives the chat `id` the title `title`.
///
/// Input: `session`, the session of the window, someone logged in; `id`, the id
/// of a chat; `title`, the title typed.
/// Output: nothing; `ViewError::NotLoggedIn` when nobody is logged in, the
/// refusals of the title.
///
/// Called by: `ui/src/lib/api/chat.ts`: `chatRename`, from `ui/src/views/chat/Conversation.svelte`, "Rename".
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/ai/chat/chat_rules.rs`: `rename_chat`.
#[tauri::command]
pub fn chat_rename(session: State<'_, Session>, id: i64, title: String) -> Result<(), ViewError> {
    ensure!(title.len() <= MAX_TITLE_BYTES, ViewError::Invalid("the title is too long".to_owned()));

    let opened = session.lock()?;

    Ok(rename_chat(database(opened.as_ref())?, id, &title)?)
}

/// The question `text` in the chat `chat`, prepared, and written into the
/// chat once it was taken; gives also how many old messages were left out.
///
/// Input: `session`, the session of the window; `chat`, the id of a chat;
/// `text`, the question typed.
/// Output: the question prepared and how many old messages were left out
/// (`(prepared, 4)`); `ViewError::NotLoggedIn` when nobody is logged in, the
/// refusals of the question.
///
/// Called by: [`chat_ask`].
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`; `objects/ai/chat/chat_ai.rs`:
/// `prepared`; `objects/ai/chat/chat_rules.rs`: `add_question`.
fn prepared_question(session: &Session, chat: i64, text: &str) -> Result<(Prepared, usize), ViewError> {
    let opened = session.lock()?;
    let db = database(opened.as_ref())?;
    let taken = prepared(db, chat, text)?;

    add_question(db, chat, text)?;

    Ok(taken)
}

/// Sends the prepared question and passes each piece of its answer to the
/// page through `on_piece`, while it comes. The answer is stopped (its
/// receiver dropped) when the user asks it to stop or the page no more
/// listens. Uses no database.
///
/// Input: `session`, the session of the window; `prepared`, the question
/// prepared; `on_piece`, the channel to the page.
/// Output: the answer as it came, readable only through
/// `objects/ai/journal/journal_ai.rs`: `record`.
///
/// Called by: [`chat_ask`].
/// Calls: `objects/ai/integration/integration_ai/sending.rs`: `send`; `panels/session.rs`:
/// `Session::stop_requested`.
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
                // A broken session stops the answer as "Stop" does.
                _ = stop_check.tick() => matches!(session.stop_requested(), Ok(false)),
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
/// `chat.0`; gives what the page shows of it, with the number `chat.1` of
/// the old messages left out of the question.
///
/// Input: `session`, the session of the window; `chat`, the id of the chat
/// and how many messages were left out; `prepared`, the question as it was
/// sent; `sent`, its answer.
/// Output: the answer as the page shows it (`{"message": {...}, "ending":
/// "complete", "leftOut": 4}`); `ViewError::NotLoggedIn` when nobody is logged
/// in, an error of the database.
///
/// Called by: [`chat_ask`].
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/ai/journal/journal_ai.rs`: `record`; `objects/ai/chat/chat_ai.rs`: `add_answer`; [`message_view`],
/// [`ending_name`].
fn written(session: &Session, chat: (i64, usize), prepared: Prepared, sent: Sent) -> Result<ChatAnswerView, ViewError> {
    let (chat, left_out) = chat;
    let opened = session.lock()?;
    let db = database(opened.as_ref())?;
    let answer = record(db, prepared, sent)?;
    let kept = add_answer(db, chat, &answer)?;

    Ok(ChatAnswerView {
        answer: kept.map(message_view),
        ending: ending_name(answer.ending),
        error: answer.error.map(|error| error.to_string()),
        left_out,
    })
}

/// Asks the question `text` in the chat `chat`: the answer is passed to the
/// page piece by piece through `on_piece`, and kept in the chat and in the
/// journal of its agent. Refused while another work of the AI runs.
///
/// Input: `session`, the session of the window, someone logged in; `chat`, the
/// id of a chat with an agent; `text`, the question typed; `on_piece`, the
/// channel to the page.
/// Output: the answer as the page shows it; `ViewError::Invalid` for a question
/// too long or while another work of the AI runs, `ViewError::NotLoggedIn` when
/// nobody is logged in, the refusals of the question, an error of the database.
///
/// Called by: `ui/src/lib/api/chat.ts`: `chatAsk`, from `ui/src/views/chat/Conversation.svelte`, "Send".
/// Calls: `panels/session.rs`: `Session::begin_work`; [`prepared_question`], [`forwarded`], [`written`].
#[tauri::command]
pub async fn chat_ask(
    session: State<'_, Session>,
    chat: i64,
    text: String,
    on_piece: Channel<String>,
) -> Result<ChatAnswerView, ViewError> {
    ensure!(text.len() <= 4 * MAX_QUESTION, ViewError::Invalid("the question is too long".to_owned()));

    let Some(_work) = session.begin_work(None, 0)? else {
        return Err(ViewError::Invalid("another work of the AI runs: wait for it or stop it".to_owned()));
    };
    let (prepared, left_out) = prepared_question(&session, chat, &text)?;
    let sent = forwarded(&session, &prepared, &on_piece).await;

    written(&session, (chat, left_out), prepared, sent)
}

/// Asks the answer that is being written to stop at once.
///
/// Called by: `ui/src/lib/api/chat.ts`: `chatStop`, from `ui/src/views/chat/Conversation.svelte`, "Stop".
/// Calls: `panels/session.rs`: `Session::request_stop`.
#[tauri::command]
pub fn chat_stop(session: State<'_, Session>) -> Result<(), ViewError> {
    Ok(session.request_stop()?)
}

#[cfg(test)]
mod tests {
    //! Tests of the forwarding of an answer: every piece reaches the page,
    //! and "Stop" ends an answer that is still being written.

    use super::*;

    use std::sync::{Arc, Mutex};

    use serde_json::json;
    use tauri::ipc::InvokeResponseBody;

    use crate::objects::ai::agent::agent_store::create_agent;
    use crate::objects::ai::integration::integration_ai::question::{Question, prepare};
    use crate::objects::ai::integration::integration_model::Integration;
    use crate::objects::ai::integration::integration_store::create_integration;
    use crate::test_support::TempDir;
    use crate::test_support::ai_service::{FakeAnswer, FakeService, openai_events, sse};
    use crate::test_support::database::{agent, database, integration};

    /// The question "Hi" of an agent to the fake service `service`.
    fn prepared(db: &UserDatabase, service: &FakeService) -> Prepared {
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
            session.request_stop().unwrap();
        };
        let forwarding = tokio::time::timeout(Duration::from_secs(10), forwarded(&session, &question, &channel));
        let (sent, ()) = tokio::join!(forwarding, stopping);
        let answer = record(&db, question, sent.unwrap()).unwrap();

        assert_eq!((answer.text.as_str(), answer.ending), ("Hal", Ending::Stopped));
        assert_eq!(*got.lock().unwrap(), ["Hal"]);
    }
}
