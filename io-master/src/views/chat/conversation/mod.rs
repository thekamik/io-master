//! The element "conversation" of the view Chat: the open chat, a new
//! message with its reply shown while it is written, and the deletion
//! (page: `ui/src/views/chat/Conversation.svelte`).
//!
//! ```text
//! action   command               parameters                 data back
//! open     conversation_open     chat_id                    ChatView: the title, the messages
//! send     conversation_send     chat_id or null, text,     (none); the reply arrives as ReplyEvent
//!                                on_event (a channel)       while it is written
//! stop     conversation_stop     (none)                     (none); the reply ends, its text is kept
//! delete   conversation_delete   chat_id                    (none)
//! ```
//!
//! ```text
//! commands.rs   the four commands
//! turn.rs       one turn: the message checked and stored, the reply asked and stored
//! stored.rs     what a chat keeps: the title of a new chat, a reply with how it ended
//! view.rs       ChatView, MessageView, ReplyEvent
//! ```

pub mod commands;
pub mod stored;
pub mod turn;
pub mod view;
