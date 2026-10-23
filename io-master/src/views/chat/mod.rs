//! The view Chat: the chats of the evaluator with the model of the role
//! *consultation* (page: `ui/src/views/chat/`).
//!
//! ```text
//! element             folder              commands
//! the connection      connection_line/    connection_line_get
//! the list of chats   chat_list/          chat_list_get
//! the open chat       conversation/       conversation_open, conversation_send,
//!                                         conversation_stop, conversation_delete
//! ```
//!
//! The flow of data. Which chat is open is presentation, kept by the page
//! of the view. The conversation tells the view when it changed the chats
//! (a message sent, a chat deleted), and the list reads them again:
//!
//! ```text
//! list ──choose(id)──► the view ──► conversation ──open(id)──► conversation_open ──► ChatView
//! conversation ──send(chat, text)──► conversation_send ──► db: the message stored
//!      ▲                                  │                 ai/api/: the reply, piece by piece
//!      └──── ReplyEvent (chat, text..., end) ◄───┘          db: the reply stored, also when stopped
//! conversation ── "changed" ──► the view ──► list ──get()──► chat_list_get
//! ```
//!
//! Every message and reply is stored by the program; the page only shows
//! them, as text, never as HTML.

pub mod chat_list;
pub mod connection_line;
pub mod conversation;
