//! The chats of the evaluator with the AI: the stored chats and one
//! turn of a chat. What of a chat goes to the model is the task `chat`
//! (`ai/tasks/chat.rs`); the question itself goes through `ai/api/`.

pub mod conversation;
pub mod history;
