//! The API key of an AI connection of one evaluator
//! (`ai/connection.rs`).
//!
//! The key is a password to an account, often a paid one: whoever
//! has it can use the account at the cost of its owner. So it is handled like the
//! key of a database (`db/database_key.rs`):
//!
//! - it is wiped from memory when it is dropped;
//! - `{:?}` shows only that there is a key, never the key itself,
//!   so it cannot end up in a log or an error message by mistake;
//! - it cannot be cloned, and its text is given out only through
//!   [`ApiKey::expose`], where a request needs it.
//!
//! It is stored only in the encrypted database of the evaluator
//! (`ai/connections.rs`), never in a file of its own or in the
//! system database.

use std::fmt;

use zeroize::Zeroizing;

/// The longest key that is accepted, in characters. API keys are
/// usually 30 to 120 characters; the limit only stops nonsense
/// such as a pasted document.
pub const MAX_LENGTH: usize = 512;

/// Why a text is not an API key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidApiKey {
    /// Nothing was entered.
    Empty,
    /// Longer than [`MAX_LENGTH`].
    TooLong,
    /// Holds a character that no API key has (a space inside the
    /// key, a letter outside ASCII, a control character). Which one
    /// is not said, so that no part of the key is repeated.
    InvalidCharacter,
}

impl fmt::Display for InvalidApiKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            InvalidApiKey::Empty => "no key was entered",
            InvalidApiKey::TooLong => "the text is too long to be an API key",
            InvalidApiKey::InvalidCharacter => "the text holds a character that an API key does not have",
        })
    }
}

impl std::error::Error for InvalidApiKey {}

/// The API key of an evaluator.
pub struct ApiKey(Zeroizing<String>);

impl ApiKey {
    /// Takes over a key as it was entered or read. Spaces and line
    /// breaks around it (often copied with it) are removed; inside
    /// the key only visible ASCII characters are allowed, which is
    /// also what an HTTP header may carry.
    ///
    /// The text is taken by value and wiped, whether it is a key or
    /// not.
    pub fn new(text: String) -> Result<ApiKey, InvalidApiKey> {
        let text = Zeroizing::new(text);
        let trimmed = text.trim();

        if trimmed.is_empty() {
            return Err(InvalidApiKey::Empty);
        }

        if trimmed.len() > MAX_LENGTH {
            return Err(InvalidApiKey::TooLong);
        }

        if !trimmed.bytes().all(|byte| byte.is_ascii_graphic()) {
            return Err(InvalidApiKey::InvalidCharacter);
        }

        // A new string of exactly the key; the original (with the
        // spaces around it) is wiped when `text` is dropped.
        Ok(ApiKey(Zeroizing::new(trimmed.to_owned())))
    }

    /// The text of the key. Use it only where the key is actually
    /// needed: in the header of a request, or to store it.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for ApiKey {
    /// Shows that there is a key, never what it is.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ApiKey(<hidden>)")
    }
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "api_key_tests.rs"]
mod tests;
