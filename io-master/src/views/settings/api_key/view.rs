//! The data of the API key, as the page gets it: never the key itself.

use serde::Serialize;

use crate::model::ai::{API_KEY, StoredConnection};

/// What the page may know of the key of a connection.
#[derive(Debug, Serialize)]
pub struct ApiKeyView {
    /// Whether a key is stored.
    pub has_key: bool,
    /// A key is stored, but no header uses `{{api_key}}`: it is never
    /// sent (the page says so).
    pub key_unused: bool,
    /// A header uses `{{api_key}}`.
    pub header_uses_key: bool,
}

impl ApiKeyView {
    pub fn new(stored: &StoredConnection) -> ApiKeyView {
        let header_uses_key = stored.connection.headers.contains(API_KEY);

        ApiKeyView { has_key: stored.has_key, key_unused: stored.has_key && !header_uses_key, header_uses_key }
    }
}
