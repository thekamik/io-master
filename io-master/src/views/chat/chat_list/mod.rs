//! The element "list of chats" of the view Chat
//! (page: `ui/src/views/chat/ChatList.svelte`).
//!
//! ```text
//! action   command         parameters   data back
//! read     chat_list_get   (none)       the chats, the newest first: id, title, when started
//! ```
//!
//! Choosing a chat and "New chat" are presentation: the page of the view
//! opens the chat in the conversation.

pub mod commands;
pub mod view;
