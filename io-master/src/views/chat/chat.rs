//! The entry of the view Chat: the chats of the evaluator with the model
//! of the role *consultation* (page: `ui/src/views/chat/`, its commands in
//! `ui/src/lib/api/chat.ts`). Which chat is open is presentation, kept by
//! the page. A reply runs while `conversation_send` waits for it; its
//! pieces go to the page through the channel `on_event` as they arrive;
//! "Stop" (`conversation_stop`) stops it, and what was written up to then
//! is stored with how it ended (`conversation/turn.rs`). Every message and
//! reply is stored by the program; the page only shows them, as text,
//! never as HTML. The flow of data: `docs/architecture.md`, section 12.

use tauri::State;
use tauri::ipc::Channel;

use crate::ai::api::connections;
use crate::app::error::AppError;
use crate::app::state::AppState;
use crate::db::ai::chats as db_chats;
use crate::db::knowledge as db_knowledge;
use crate::model_old::knowledge::AgentPlace;

use super::chat_list::view::ChatSummaryView;
use super::connection_line::view::ConnectionLineView;
use super::conversation::turn;
use super::conversation::view::{ChatView, ReplyEvent};

// ── When the view opens ───────────────────────────────────────────────

/// The connection the chat uses now, or `None` when none is chosen.
///
/// Called by the page: `ConnectionLine.svelte`, when the view opens
/// (`chat.connectionLineGet()`).
/// Calls: `connections::active`, `connections::get`, `ConnectionLineView::new`.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn connection_line_get(state: State<'_, AppState>) -> Result<Option<ConnectionLineView>, AppError> {
    let session = state.session()?;
    let db = session.db();

    let Some(id) = connections::active(&db)? else {
        return Ok(None);
    };

    let Some(stored) = connections::get(&db, id)? else {
        return Ok(None);
    };

    Ok(Some(ConnectionLineView::new(stored.connection.name, stored.connection.model)))
}

/// The chats of the evaluator, the newest first.
///
/// Called by the page: `ChatList.svelte`, when the view opens and when the
/// conversation changed the chats (`chat.chatListGet()`).
/// Calls: `db_chats::read::chats`, `ChatSummaryView::all`.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn chat_list_get(state: State<'_, AppState>) -> Result<Vec<ChatSummaryView>, AppError> {
    let session = state.session()?;
    let chats = db_chats::read::chats(&session.db())?;

    Ok(ChatSummaryView::all(chats))
}

// ── The actions of the elements ───────────────────────────────────────

/// Chat `chat_id` with its messages.
///
/// Called by the page: `Conversation.svelte`, when a chat is chosen in the
/// list (`chat.conversationOpen(id)`).
/// Calls: `db_chats::read::chat_title`, `db_chats::read::messages`,
/// `ChatView::new`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn conversation_open(state: State<'_, AppState>, chat_id: i64) -> Result<ChatView, AppError> {
    let session = state.session()?;
    let db = session.db();

    let Some(title) = db_chats::read::chat_title(&db, chat_id)? else {
        return Err(AppError::Invalid("the chat does not exist".to_owned()));
    };

    let agent = db_knowledge::read::attachment(&db, AgentPlace::Chat, &chat_id.to_string())?;

    Ok(ChatView::new(chat_id, title, db_chats::read::messages(&db, chat_id)?, agent))
}

/// Sends `text` to chat `chat_id` (a new chat when `None`) and waits for
/// the reply; its events go to `on_event` while it is written
/// (`turn.rs`). `agent` is the agent with knowledge the evaluator chose
/// for the chat (none: `None`): it is kept with the chat, and its passages
/// go after the question (step 57.6). One reply at a time: a reply still being written is
/// stopped first.
///
/// Called by the page: `Conversation.svelte`, "Send"
/// (`chat.conversationSend(...)`).
/// Calls: `AppState::start_reply` (the stop signal), `turn::send` (the
/// message stored, the question asked through `ai/api/`, the reply stored).
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn conversation_send(
    state: State<'_, AppState>,
    chat_id: Option<i64>,
    text: String,
    agent: Option<i64>,
    on_event: Channel<ReplyEvent>,
) -> Result<(), AppError> {
    let session = state.session()?;
    let stop = state.start_reply();

    turn::send(&session, chat_id, &text, agent, &stop, &on_event).await?;

    Ok(())
}

/// Stops the reply being written, if there is one.
///
/// Called by the page: `Conversation.svelte`, "Stop" (`chat.conversationStop()`).
/// Calls: `AppState::stop_reply`.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn conversation_stop(state: State<'_, AppState>) {
    state.stop_reply();
}

/// Deletes chat `chat_id` with its messages; the text is overwritten in
/// the file (`db/ai/chats/delete.rs`).
///
/// Called by the page: `Conversation.svelte`, "Delete"
/// (`chat.conversationDelete(id)`).
/// Calls: `db_chats::delete::chat`.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn conversation_delete(state: State<'_, AppState>, chat_id: i64) -> Result<(), AppError> {
    let session = state.session()?;

    ensure!(db_chats::delete::chat(&session.db(), chat_id)?, AppError::Invalid("the chat does not exist".to_owned()));

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Tests of the commands of the view Chat, called as the page calls
    //! them: by name, through the IPC of Tauri. A message with its reply is
    //! tested in `conversation/turn.rs`: its events go through a channel,
    //! which the mock window does not carry.

    use serde_json::json;

    use crate::ai::api::connections;
    use crate::model_old::ai::Connection;
    use crate::views::chat::conversation::stored;
    use crate::views::test_support::{logged_in_window, logged_out_window};

    #[test]
    fn without_a_connection_the_line_is_empty() {
        assert_eq!(logged_in_window().call("connection_line_get", json!({})).unwrap(), json!(null));
    }

    #[test]
    fn the_line_names_the_active_connection_and_its_model() {
        let window = logged_in_window();
        let connection = Connection { name: "Local".to_owned(), model: "llama".to_owned(), ..Connection::template() };
        connections::create(&window.session().db(), &connection).unwrap();

        let line = window.call("connection_line_get", json!({})).unwrap();

        assert_eq!(line, json!({ "name": "Local", "writer": "llama" }));
    }

    #[test]
    fn the_chats_are_listed_the_newest_first() {
        let window = logged_in_window();
        let first = stored::start(&window.session().db(), "first").unwrap();
        let second = stored::start(&window.session().db(), "second").unwrap();

        let chats = window.call("chat_list_get", json!({})).unwrap();

        assert_eq!(chats[0]["id"], second);
        assert_eq!(chats[0]["title"], "second");
        assert_eq!(chats[1]["id"], first);
    }

    #[test]
    fn the_list_needs_a_login() {
        assert_eq!(logged_out_window().call("chat_list_get", json!({})).unwrap_err()["kind"], "not_logged_in");
    }

    #[test]
    fn a_chat_is_opened_and_deleted() {
        let window = logged_in_window();
        let id = stored::start(&window.session().db(), "What is ADV_FSP?").unwrap();

        let chat = window.call("conversation_open", json!({ "chatId": id })).unwrap();
        assert_eq!(chat["title"], "What is ADV_FSP?");
        assert_eq!(
            chat["messages"][0],
            json!({
                "speaker": "evaluator", "who": "You", "text": "What is ADV_FSP?",
                "created": chat["messages"][0]["created"], "notice": null
            })
        );

        window.call("conversation_delete", json!({ "chatId": id })).unwrap();

        assert_eq!(window.call("conversation_open", json!({ "chatId": id })).unwrap_err()["kind"], "invalid");
        assert_eq!(window.call("conversation_delete", json!({ "chatId": id })).unwrap_err()["kind"], "invalid");
        assert_eq!(window.call("chat_list_get", json!({})).unwrap(), json!([]));
    }

    #[test]
    fn stop_without_a_reply_does_nothing() {
        logged_in_window().call("conversation_stop", json!({})).unwrap();
    }

    #[test]
    fn the_chats_need_a_login() {
        let window = logged_out_window();

        assert_eq!(window.call("conversation_open", json!({ "chatId": 1 })).unwrap_err()["kind"], "not_logged_in");
    }
}
