//! The data of the list of connections, as the page gets it.

use serde::Serialize;

use crate::model_old::ai::StoredConnection;

/// A connection in the list.
#[derive(Debug, Serialize)]
pub struct ConnectionSummaryView {
    pub id: i64,
    pub name: String,
}

/// The list of connections.
#[derive(Debug, Serialize)]
pub struct ConnectionListView {
    /// The connections, by name.
    pub connections: Vec<ConnectionSummaryView>,
    /// The connection the chat uses.
    pub active: Option<i64>,
}

impl ConnectionListView {
    pub fn new(stored: Vec<StoredConnection>, active: Option<i64>) -> ConnectionListView {
        let mut connections = Vec::with_capacity(stored.len());

        for connection in stored {
            connections.push(ConnectionSummaryView { id: connection.id, name: connection.connection.name });
        }

        ConnectionListView { connections, active }
    }
}
