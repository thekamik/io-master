//! Help for the tests of `db/`: an AI connection that keeps the rules,
//! and the API key of the tests.

use crate::model::ai::Connection;
use crate::model::api_key::ApiKey;

/// The API key of the tests; it must never be found in the file of the
/// database.
pub const SECRET_KEY: &str = "sk-very-secret-key-of-anna";

/// A connection named `name` that keeps the rules.
pub fn named(name: &str) -> Connection {
    Connection { name: name.to_owned(), model: "llama".to_owned(), ..Connection::template() }
}

/// The API key of the tests.
pub fn key() -> ApiKey {
    ApiKey::new(SECRET_KEY.to_owned()).unwrap()
}
