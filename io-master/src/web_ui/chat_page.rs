//! The chat page: conversations of the evaluator with a Claude
//! model, and the settings of the chat.
//!
//! ```text
//! GET  /chat               the list of chats and an empty new chat
//! GET  /chat/{id}          a chat with its messages
//! POST /chat/send          a message; the reply streams back
//! POST /chat/{id}/delete   delete a chat
//! POST /chat/settings      check and store the chosen model
//! ```
//!
//! **Replies are shown while they are written.** The page sends a
//! message with a small script (`static/chat.js`, served by the
//! program itself, the only kind of script the Content-Security-
//! Policy allows). `POST /chat/send` answers with a stream of
//! server-sent events:
//!
//! ```text
//! event: chat    {"id": 7, "url": "/chat/7", "title": "..."}   first
//! event: text    {"text": "a piece of the reply"}             many
//! event: end     {"ending": "complete", "notice": "..."}       last
//! ```
//!
//! The work runs in a task of its own, apart from the connection
//! to the browser: when the browser goes away (the evaluator
//! pressed "Stop" or left the page), the reply is stopped, and what
//! was written up to then is still stored. Every message and reply
//! is stored in the encrypted database of the evaluator
//! (`ai/chats.rs`).

use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::{Extension, Form};
use futures_util::stream;
use serde::Deserialize;
use serde_json::json;
use tokio::sync::mpsc;

use crate::accounts::accounts::Account;
use crate::ai::ai_error::AiError;
use crate::ai::ai_settings;
use crate::ai::anthropic::Speaker;
use crate::ai::api_key::ApiKey;
use crate::ai::chats::{self, ChatMessage, ChatSummary, Ending};
use crate::ai::model::Model;
use crate::db::database_key::DatabaseKey;
use crate::db::db::DbError;
use crate::db::user_db::{self, UserDb};
use crate::installation::lifecycle::Running;

use super::pages::{self, Page, escape};
use super::session::{CurrentUser, UserKey};
use super::web_ui::{AppState, not_found};

/// The longest message an evaluator can send, in characters. A
/// longer text is better given to the model as a document (a later
/// step).
const MAX_MESSAGE: usize = 100_000;

/// How many events may wait for the browser before the reply waits
/// for it.
const EVENT_QUEUE: usize = 64;

/// The running installation, when it is a workstation: only a
/// workstation has this page.
fn workstation(state: &AppState) -> Option<Arc<Running>> {
    state
        .running()
        .filter(|running| Page::of_role(running.installation.role).contains(&Page::Chat))
}

/// Runs work on the database of the evaluator on a thread for
/// blocking work; an interrupted thread is reported as a database
/// error.
async fn with_db<T: Send + 'static>(
    state: &AppState,
    account: &Account,
    key: Arc<DatabaseKey>,
    work: impl FnOnce(&UserDb) -> Result<T, DbError> + Send + 'static,
) -> Result<T, DbError> {
    let data_dir: PathBuf = state.data_dir.as_ref().clone();
    let account_id = account.id;

    tokio::task::spawn_blocking(move || work(&user_db::open(&data_dir, account_id, &key)?))
        .await
        .unwrap_or_else(|_| Err(DbError::Sql("the work was interrupted".to_owned())))
}

/// What the page shows, read from the database.
struct PageData {
    model: Model,
    has_key: bool,
    chats: Vec<ChatSummary>,
    /// The open chat: its number, title and messages.
    open: Option<(i64, String, Vec<ChatMessage>)>,
}

/// Reads what the page shows; `Ok(None)` when the chat asked for
/// does not exist.
fn read_page(db: &UserDb, chat_id: Option<i64>) -> Result<Option<PageData>, DbError> {
    let open = match chat_id {
        None => None,
        Some(id) => match chats::title(db, id)? {
            None => return Ok(None),
            Some(title) => Some((id, title, chats::messages(db, id)?)),
        },
    };

    Ok(Some(PageData {
        model: ai_settings::model(db)?,
        has_key: ai_settings::has_api_key(db)?,
        chats: chats::list(db)?,
        open,
    }))
}

/// `GET /chat`: the list of chats and a new, empty chat.
pub async fn show_new(
    State(state): State<AppState>,
    Extension(CurrentUser(account)): Extension<CurrentUser>,
    Extension(UserKey(key)): Extension<UserKey>,
) -> Response {
    show(state, account, key, None, None).await
}

/// `GET /chat/{id}`: a chat.
pub async fn show_chat(
    State(state): State<AppState>,
    Extension(CurrentUser(account)): Extension<CurrentUser>,
    Extension(UserKey(key)): Extension<UserKey>,
    Path(chat_id): Path<i64>,
) -> Response {
    show(state, account, key, Some(chat_id), None).await
}

/// The page with an optional chat and notice.
async fn show(
    state: AppState,
    account: Account,
    key: Arc<DatabaseKey>,
    chat_id: Option<i64>,
    notice: Option<(StatusCode, String)>,
) -> Response {
    let Some(running) = workstation(&state) else {
        return not_found().await;
    };

    let (status, content) = match with_db(&state, &account, key, move |db| read_page(db, chat_id)).await {
        Ok(Some(data)) => match notice {
            Some((status, text)) => (status, content(&data, Some(&text))),
            None => (StatusCode::OK, content(&data, None)),
        },
        Ok(None) => return not_found().await,
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            pages::problem_text(Some(&format!("Your chats cannot be read: {error}."))),
        ),
    };

    let html = pages::page(&running.installation, &account.name, Page::Chat, &content);

    (status, Html(html)).into_response()
}

/// The HTML of the page: the folded settings, the list of chats,
/// the open chat and the form to write in it.
fn content(data: &PageData, problem: Option<&str>) -> String {
    let problem = problem.map(|text| pages::problem_text(Some(text))).unwrap_or_default();

    let no_key = if data.has_key {
        String::new()
    } else {
        "<p class=\"problem\">No API key of Anthropic is stored. Enter one in \
         <a href=\"/settings\">Settings</a> to use the chat.</p>"
            .to_owned()
    };

    let open_id = data.open.as_ref().map(|(id, _, _)| *id);

    let (title, messages, delete) = match &data.open {
        Some((id, title, messages)) => (
            escape(title),
            messages.iter().map(message_html).collect::<String>(),
            format!(
                "<form method=\"post\" action=\"/chat/{id}/delete\" class=\"inline\">\
                 <button type=\"submit\">Delete this chat</button></form>"
            ),
        ),
        None => ("New chat".to_owned(), String::new(), String::new()),
    };

    format!(
        "{settings}\n\
         {problem}{no_key}\n\
         <div class=\"chat\">\n\
         <aside class=\"chat-list\">\n\
         <a class=\"button\" href=\"/chat\">New chat</a>\n\
         <ul id=\"chats\">{list}</ul>\n\
         </aside>\n\
         <section class=\"conversation\">\n\
         <h2><span id=\"chat-title\">{title}</span> {delete}</h2>\n\
         <div id=\"messages\">{messages}</div>\n\
         <form id=\"ask\" method=\"post\" action=\"/chat/send\">\n\
         <input type=\"hidden\" name=\"chat\" value=\"{chat}\">\n\
         <textarea name=\"text\" rows=\"4\" maxlength=\"{MAX_MESSAGE}\" required \
         placeholder=\"Write to {model}\"></textarea>\n\
         <div class=\"buttons\">\n\
         <button type=\"submit\" id=\"send\">Send</button>\n\
         <button type=\"button\" id=\"stop\" hidden>Stop</button>\n\
         <span id=\"status\" class=\"note\"></span>\n\
         </div>\n\
         </form>\n\
         <noscript><p class=\"problem\">The chat needs JavaScript to show the replies \
         while they are written.</p></noscript>\n\
         </section>\n\
         </div>\n\
         <script src=\"/static/chat.js\" defer></script>",
        settings = settings_form(data.model),
        list = data.chats.iter().map(|chat| chat_link(chat, open_id)).collect::<String>(),
        chat = open_id.map(|id| id.to_string()).unwrap_or_default(),
        model = escape(data.model.title()),
    )
}

/// The folded settings of the chat: the model.
fn settings_form(chosen: Model) -> String {
    let choices: String = Model::ALL
        .iter()
        .map(|model| {
            format!(
                "<label class=\"choice\"><input type=\"radio\" name=\"model\" value=\"{id}\"{checked}> \
                 <strong>{title}</strong>: {description}</label>",
                id = model.id(),
                checked = if *model == chosen { " checked" } else { "" },
                title = escape(model.title()),
                description = escape(model.description()),
            )
        })
        .collect();

    format!(
        "<details class=\"settings\">\n\
         <summary>Chat settings: {title}</summary>\n\
         <form method=\"post\" action=\"/chat/settings\">\n\
         <fieldset>\n\
         <legend>Model</legend>\n\
         <div class=\"models\">{choices}</div>\n\
         </fieldset>\n\
         <p class=\"note\">The model is checked with your API key before it is saved. With \
         {opus}, Anthropic may repeat a request that its safety checks declined on another \
         Claude model (server-side fallback), so that a question about security is not refused \
         by mistake; the reply then names the model that wrote it.</p>\n\
         <button type=\"submit\">Check and save</button>\n\
         </form>\n\
         </details>",
        title = escape(chosen.title()),
        opus = escape(Model::Opus.title()),
    )
}

/// A chat in the list on the left.
fn chat_link(chat: &ChatSummary, open: Option<i64>) -> String {
    format!(
        "<li><a href=\"/chat/{id}\"{active}>{title}</a><span class=\"note\">{created}</span></li>",
        id = chat.id,
        active = if open == Some(chat.id) { " class=\"active\"" } else { "" },
        title = escape(&chat.title),
        created = escape(&chat.created),
    )
}

/// What the page says under a reply that did not end normally;
/// `None` for a complete reply. Also sent to the script at the end
/// of a streamed reply, so that both say the same.
fn ending_notice(ending: &Ending, note: Option<&str>) -> Option<String> {
    match ending {
        Ending::Complete => None,
        Ending::Cut => Some("The reply reached its largest length and was cut.".to_owned()),
        Ending::Stopped => Some("Stopped.".to_owned()),
        Ending::Refused => Some("The model declined to answer this request.".to_owned()),
        Ending::Failed => Some(format!("The reply failed: {}.", note.unwrap_or("no explanation"))),
    }
}

/// A message of the chat. The text is escaped and keeps its lines
/// (the style sheet shows it as written); it is never read as HTML.
fn message_html(message: &ChatMessage) -> String {
    let (class, who) = match message.speaker {
        Speaker::Evaluator => ("evaluator", "You".to_owned()),
        Speaker::Model => ("model", message.model.clone().unwrap_or_else(|| "Model".to_owned())),
    };

    let notice = ending_notice(&message.ending, message.note.as_deref())
        .map(|text| format!("<p class=\"ending\">{}</p>", escape(&text)))
        .unwrap_or_default();

    format!(
        "<article class=\"message {class}\">\
         <header>{who} · {created}</header>\
         <div class=\"text\">{text}</div>{notice}\
         </article>",
        who = escape(&who),
        created = escape(&message.created),
        text = escape(&message.text),
    )
}

/// `POST /chat/{id}/delete`: deletes a chat, then back to a new
/// one.
pub async fn delete(
    State(state): State<AppState>,
    Extension(CurrentUser(account)): Extension<CurrentUser>,
    Extension(UserKey(key)): Extension<UserKey>,
    Path(chat_id): Path<i64>,
) -> Response {
    if workstation(&state).is_none() {
        return not_found().await;
    }

    match with_db(&state, &account, Arc::clone(&key), move |db| chats::delete(db, chat_id)).await {
        Ok(true) => Redirect::to("/chat").into_response(),
        Ok(false) => not_found().await,
        Err(error) => {
            let notice = (StatusCode::INTERNAL_SERVER_ERROR, format!("The chat was not deleted: {error}."));

            show(state, account, key, Some(chat_id), Some(notice)).await
        }
    }
}

/// The field of the settings form.
#[derive(Deserialize)]
pub struct SettingsForm {
    model: String,
}

/// `POST /chat/settings`: checks the chosen model with the stored
/// key, as in v0.2, and stores it when the check passes.
pub async fn save_settings(
    State(state): State<AppState>,
    Extension(CurrentUser(account)): Extension<CurrentUser>,
    Extension(UserKey(key)): Extension<UserKey>,
    Form(form): Form<SettingsForm>,
) -> Response {
    if workstation(&state).is_none() {
        return not_found().await;
    }

    let Some(model) = Model::from_id(&form.model) else {
        let notice = (StatusCode::BAD_REQUEST, "Choose one of the models of the list.".to_owned());

        return show(state, account, key, None, Some(notice)).await;
    };

    let refused = |status: StatusCode, text: String| Some((status, format!("The model was not saved: {text}.")));

    let notice = match with_db(&state, &account, Arc::clone(&key), ai_settings::api_key).await {
        Ok(None) => refused(StatusCode::BAD_REQUEST, AiError::NoKey.to_string()),
        Err(error) => refused(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
        Ok(Some(api_key)) => match state.ai.check(&api_key, model).await {
            Err(error) => refused(StatusCode::BAD_REQUEST, error.to_string()),
            Ok(()) => {
                drop(api_key);

                match with_db(&state, &account, Arc::clone(&key), move |db| ai_settings::store_model(db, model)).await {
                    Ok(()) => None,
                    Err(error) => refused(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
                }
            }
        },
    };

    match notice {
        None => Redirect::to("/chat").into_response(),
        Some(notice) => show(state, account, key, None, Some(notice)).await,
    }
}

/// The fields of the form that sends a message.
#[derive(Deserialize)]
pub struct SendForm {
    /// The number of the chat; empty for a new chat.
    #[serde(default)]
    chat: String,
    text: String,
}

/// What [`prepare`] makes ready for a reply.
struct Prepared {
    chat_id: i64,
    title: String,
    api_key: ApiKey,
    model: Model,
    messages: Vec<ChatMessage>,
}

/// Why a message could not be sent.
enum SendProblem {
    NoKey,
    NoSuchChat,
    Db(DbError),
}

/// Stores the message of the evaluator (in a new chat when there is
/// none yet) and reads what the reply needs. Runs on a thread for
/// blocking work.
fn prepare(db: &UserDb, chat_id: Option<i64>, text: &str) -> Result<Prepared, SendProblem> {
    let api_key = ai_settings::api_key(db).map_err(SendProblem::Db)?.ok_or(SendProblem::NoKey)?;
    let model = ai_settings::model(db).map_err(SendProblem::Db)?;

    let chat_id = match chat_id {
        Some(id) => {
            chats::title(db, id).map_err(SendProblem::Db)?.ok_or(SendProblem::NoSuchChat)?;
            chats::add_message(db, id, text).map_err(SendProblem::Db)?;
            id
        }
        None => chats::start(db, text).map_err(SendProblem::Db)?,
    };

    Ok(Prepared {
        chat_id,
        title: chats::title(db, chat_id).map_err(SendProblem::Db)?.unwrap_or_default(),
        api_key,
        model,
        messages: chats::messages(db, chat_id).map_err(SendProblem::Db)?,
    })
}

/// An answer to the script that is not a stream: a status and
/// a sentence, which the script shows.
fn refuse(status: StatusCode, text: &str) -> Response {
    (status, text.to_owned()).into_response()
}

/// `POST /chat/send`: stores the message and streams the reply.
pub async fn send(
    State(state): State<AppState>,
    Extension(CurrentUser(account)): Extension<CurrentUser>,
    Extension(UserKey(key)): Extension<UserKey>,
    Form(form): Form<SendForm>,
) -> Response {
    if workstation(&state).is_none() {
        return not_found().await;
    }

    let text = form.text.trim().to_owned();

    if text.is_empty() {
        return refuse(StatusCode::BAD_REQUEST, "The message is empty.");
    }

    if text.chars().count() > MAX_MESSAGE {
        return refuse(StatusCode::BAD_REQUEST, "The message is too long.");
    }

    let chat_id = match form.chat.trim() {
        "" => None,
        number => match number.parse::<i64>() {
            Ok(id) => Some(id),
            Err(_) => return refuse(StatusCode::BAD_REQUEST, "The chat is not known."),
        },
    };

    let data_dir = state.data_dir.as_ref().clone();
    let account_id = account.id;
    let worker_key = Arc::clone(&key);

    let prepared = tokio::task::spawn_blocking(move || {
        let db = user_db::open(&data_dir, account_id, &worker_key).map_err(SendProblem::Db)?;

        prepare(&db, chat_id, &text)
    })
    .await;

    let prepared = match prepared {
        Ok(Ok(prepared)) => prepared,
        Ok(Err(SendProblem::NoKey)) => {
            return refuse(StatusCode::BAD_REQUEST, "No API key is stored; enter one in Settings.");
        }
        Ok(Err(SendProblem::NoSuchChat)) => return refuse(StatusCode::NOT_FOUND, "The chat does not exist."),
        Ok(Err(SendProblem::Db(error))) => {
            return refuse(StatusCode::INTERNAL_SERVER_ERROR, &format!("The message was not stored: {error}."));
        }
        Err(_) => return refuse(StatusCode::INTERNAL_SERVER_ERROR, "The work was interrupted."),
    };

    let (events, receiver) = mpsc::channel::<Event>(EVENT_QUEUE);

    // The first event: which chat the message went to, so that the
    // script can show it in the list and the address bar.
    let chat_event = json!({
        "id": prepared.chat_id,
        "url": format!("/chat/{}", prepared.chat_id),
        "title": prepared.title,
    });
    let _ = events.send(Event::default().event("chat").data(chat_event.to_string())).await;

    tokio::spawn(write_reply(state, account, key, prepared, events));

    // The stream ends when the task drops its sender.
    let stream = stream::unfold(receiver, |mut receiver| async move {
        receiver.recv().await.map(|event| (Ok::<_, Infallible>(event), receiver))
    });

    Sse::new(stream).into_response()
}

/// The task that asks the model and stores the reply. It passes
/// the pieces of the reply on to the browser; when the browser goes
/// away it stops the reply, which is then stored as it is.
async fn write_reply(
    state: AppState,
    account: Account,
    key: Arc<DatabaseKey>,
    prepared: Prepared,
    events: mpsc::Sender<Event>,
) {
    let Prepared {
        chat_id,
        api_key,
        model,
        messages,
        ..
    } = prepared;

    let turns = chats::turns(&messages);
    let (pieces, mut piece_receiver) = mpsc::channel::<String>(EVENT_QUEUE);

    // Asks the model; `pieces` is dropped when the reply is done,
    // which ends the passing on below.
    let ai = state.ai.clone();
    let asking = async move {
        let reply = ai.reply(&api_key, model, &turns, &pieces).await;
        drop(pieces);
        reply
    };

    // Passes the pieces on to the browser. When the browser is
    // gone, the receiver of the pieces is dropped, and that stops
    // the reply (see ai/anthropic.rs).
    let browser = events.clone();
    let passing_on = async move {
        loop {
            tokio::select! {
                piece = piece_receiver.recv() => match piece {
                    Some(piece) => {
                        let event = Event::default().event("text").data(json!({"text": piece}).to_string());

                        if browser.send(event).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                },
                () = browser.closed() => break,
            }
        }
    };

    let (reply, ()) = tokio::join!(asking, passing_on);

    let (ending, note) = chats::ending_of(&reply);

    let stored = with_db(&state, &account, key, move |db| chats::add_reply(db, chat_id, &reply)).await;

    let mut notice = ending_notice(&ending, note.as_deref());

    if let Err(error) = stored {
        let text = format!("The reply was not stored: {error}.");
        notice = Some(notice.map(|before| format!("{before} {text}")).unwrap_or(text));
    }

    // The browser may be gone already; then nobody reads this.
    let end = json!({"ending": ending.name(), "notice": notice});
    let _ = events.send(Event::default().event("end").data(end.to_string())).await;
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "chat_page_tests.rs"]
mod tests;
