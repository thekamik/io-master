//! The data of the connection line, as the page gets it.

use serde::Serialize;

/// The connection the chat uses.
#[derive(Debug, Serialize)]
pub struct ConnectionLineView {
    /// The name the evaluator gave the connection.
    pub name: String,
    /// Who writes the replies, as the page names it: the model, or the
    /// name of the connection when it names no model.
    pub writer: String,
}

impl ConnectionLineView {
    pub fn new(name: String, model: String) -> ConnectionLineView {
        let writer = if model.is_empty() { name.clone() } else { model };

        ConnectionLineView { name, writer }
    }
}
