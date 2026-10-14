//! Storage of critical data: API keys, and later any other secret
//! the program works with.
//!
//! Secrets are never written to the files of the program. They
//! are kept by the keyring of the system, the *Secret Service* of
//! the desktop (GNOME Keyring, KWallet and compatible services),
//! which stores them encrypted and unlocks them when the user
//! logs in.
//!
//! This module knows nothing about user interfaces. The console
//! and the web UI both get the secret from the user in their own
//! way (a hidden prompt, a form sent over HTTPS), turn it into
//! a [`Secret`] and call [`SecretStore::store`].
//!
//! ```text
//! let key = Secret::api_key(typed_text)?;       // checks and trims
//! let name = SecretName::new("anthropic")?;
//! SystemKeyring.store(&name, &key)?;
//! ...
//! let key = SystemKeyring.load(&name)?;         // when it is needed
//! send_request(key.expose());
//! ```
//!
//! How a secret is protected
//! -------------------------
//! - **In memory:** a [`Secret`] overwrites its text with zeros
//!   when it is dropped, and never shows it in debug output or
//!   error messages. Only [`Secret::expose`] gives the text out.
//! - **On the way to the keyring:** the Secret Service session is
//!   encrypted (Diffie-Hellman key exchange, AES), so the secret
//!   does not cross the D-Bus connection as plain text.
//! - **On disk:** the keyring service encrypts it.
//!
//! What the keyring does *not* protect against: other programs of
//! the same user while the keyring is unlocked. That is the limit
//! of every desktop keyring.
//!
//! Entries in the keyring
//! ----------------------
//! Every entry carries two attributes, the same ones v0.2 used,
//! so keys stored by v0.2 are found as well:
//!
//! ```text
//! application = io-master
//! key_name    = <name>        for example "anthropic"
//! ```
//!
//! They can be looked at with the tools of the system, for example
//! `secret-tool search application io-master`.
//!
//! The key that encrypts the database is one of these secrets;
//! its own rules are in `database_key.rs`.

// TODO: Remove when the user interfaces call this module; until
//       then the compiler sees no user of it.
#![allow(dead_code)]

use std::collections::HashMap;
use std::fmt;

use secret_service::EncryptionType;
use secret_service::blocking::{Collection, Item, SecretService};
use zeroize::Zeroizing;

/* ---------------------------------------------------------------
 * Names and secrets
 * ------------------------------------------------------------- */

/// Value of the "application" attribute of every entry.
const APPLICATION: &str = "io-master";

/// The longest name of an entry, in bytes.
const NAME_MAX: usize = 64;

/// The longest secret, in bytes. Generous for keys and tokens;
/// a limit keeps a mistaken paste of a whole file out.
const SECRET_MAX: usize = 16 * 1024;

/// The name of a secret in the keyring, for example `anthropic`.
///
/// A name has 1 to 64 characters: letters, digits, `.`, `_` and
/// `-`. The limit keeps names readable in keyring managers and
/// free of anything that could be mistaken for a separator.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SecretName(String);

impl SecretName {
    /// Checks a name.
    pub fn new(name: &str) -> Result<SecretName, KeyringError> {
        if name.is_empty() || name.len() > NAME_MAX {
            return Err(KeyringError::InvalidName(format!(
                "a name has 1 to {NAME_MAX} characters"
            )));
        }

        let allowed = |character: char| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        };

        if !name.chars().all(allowed) {
            return Err(KeyringError::InvalidName(
                "a name may contain only letters, digits, '.', '_' and '-'".to_owned(),
            ));
        }

        Ok(SecretName(name.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SecretName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// A secret held in memory.
///
/// The text is overwritten with zeros when the `Secret` is
/// dropped, including any spare memory the text once used. It is
/// not shown by `{:?}`; only [`Secret::expose`] gives it out, so
/// every place that uses the plain text is easy to find.
///
/// There is deliberately no `Clone`: a secret should exist in as
/// few copies as possible.
pub struct Secret(Zeroizing<String>);

impl Secret {
    /// Takes a secret as it is, for example a token or a password.
    ///
    /// The text is taken over, not copied. It must not be empty
    /// and not longer than 16 KiB.
    pub fn new(text: String) -> Result<Secret, KeyringError> {
        // From here on the text is wiped when dropped, also when
        // a check below refuses it.
        let text = Zeroizing::new(text);

        if text.is_empty() {
            return Err(KeyringError::InvalidSecret("the secret is empty".to_owned()));
        }

        if text.len() > SECRET_MAX {
            return Err(KeyringError::InvalidSecret(format!(
                "the secret is longer than {SECRET_MAX} bytes"
            )));
        }

        Ok(Secret(text))
    }

    /// Takes an API key as typed or pasted by the user.
    ///
    /// Spaces and line ends around the key are removed: a paste
    /// often brings them along. What remains must be printable
    /// ASCII without spaces, which every API key is; anything
    /// else means that something other than the key was pasted.
    pub fn api_key(mut text: String) -> Result<Secret, KeyringError> {
        // Trimmed in place, so no second copy of the key is made.
        let end = text.trim_end().len();
        text.truncate(end);

        let start = text.len() - text.trim_start().len();
        text.drain(..start);

        let secret = Secret::new(text)?;

        if !secret.expose().chars().all(|character| character.is_ascii_graphic()) {
            return Err(KeyringError::InvalidSecret(
                "an API key may contain only printable characters without spaces".to_owned(),
            ));
        }

        Ok(secret)
    }

    /// The text of the secret. Use it only where the secret is
    /// actually needed, for example in the header of a request.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    /// Shows that there is a secret, never what it is.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Secret(<hidden>)")
    }
}

/* ---------------------------------------------------------------
 * Errors
 * ------------------------------------------------------------- */

/// Why a keyring operation failed. No variant ever contains
/// a secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyringError {
    /// The name of the entry is not valid; holds the reason.
    InvalidName(String),
    /// The secret is not valid; holds the reason.
    InvalidSecret(String),
    /// No secret is stored under that name.
    NotFound,
    /// A secret is already stored under that name and must not
    /// be replaced (see `database_key.rs`).
    AlreadyExists,
    /// No keyring service is running, or it cannot be reached
    /// (for example in a session without a desktop).
    Unavailable,
    /// The keyring is locked and the user did not unlock it: the
    /// unlock dialog was closed or cancelled.
    Dismissed,
    /// The keyring service reported another problem; holds its
    /// description.
    Failed(String),
}

impl fmt::Display for KeyringError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeyringError::InvalidName(reason) => write!(formatter, "invalid name: {reason}"),
            KeyringError::InvalidSecret(reason) => write!(formatter, "invalid secret: {reason}"),
            KeyringError::NotFound => formatter.write_str("no secret is stored under that name"),
            KeyringError::AlreadyExists => {
                formatter.write_str("a secret is already stored under that name")
            }
            KeyringError::Unavailable => {
                formatter.write_str("the keyring of the system cannot be reached")
            }
            KeyringError::Dismissed => formatter.write_str("the keyring was not unlocked"),
            KeyringError::Failed(description) => write!(formatter, "keyring error: {description}"),
        }
    }
}

impl std::error::Error for KeyringError {}

impl From<secret_service::Error> for KeyringError {
    fn from(error: secret_service::Error) -> KeyringError {
        use secret_service::Error;

        match error {
            Error::Unavailable => KeyringError::Unavailable,
            Error::Prompt | Error::PromptDisconnected | Error::Locked => KeyringError::Dismissed,
            // Messages of the service and of D-Bus describe the
            // call, never the secret it carried.
            other => KeyringError::Failed(other.to_string()),
        }
    }
}

/* ---------------------------------------------------------------
 * The store
 * ------------------------------------------------------------- */

/// A place that keeps secrets under names.
///
/// [`SystemKeyring`] is the real one. The trait lets the rest of
/// the program, and its tests, work with any store.
pub trait SecretStore {
    /// Stores a secret under a name, replacing a secret that is
    /// already stored there.
    fn store(&self, name: &SecretName, secret: &Secret) -> Result<(), KeyringError>;

    /// Reads the secret stored under a name.
    fn load(&self, name: &SecretName) -> Result<Secret, KeyringError>;

    /// Removes the secret stored under a name.
    fn delete(&self, name: &SecretName) -> Result<(), KeyringError>;

    /// Tells whether a secret is stored under a name, without
    /// reading it. Does not ask the user to unlock the keyring.
    fn contains(&self, name: &SecretName) -> Result<bool, KeyringError>;
}

/// The keyring of the system, reached through the Secret Service
/// over D-Bus.
///
/// Every call opens its own connection. Secrets are needed
/// rarely, and a short connection holds no state that could go
/// stale, for example when the keyring is locked meanwhile.
///
/// When the keyring is locked, the service shows its own unlock
/// dialog on the desktop and the call waits for the user.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemKeyring;

impl SystemKeyring {
    /// Opens a connection with an encrypted session.
    fn connect<'a>() -> Result<SecretService<'a>, KeyringError> {
        Ok(SecretService::connect(EncryptionType::Dh)?)
    }

    /// The attributes that identify the entry of a name.
    fn attributes(name: &SecretName) -> HashMap<&str, &str> {
        HashMap::from([("application", APPLICATION), ("key_name", name.as_str())])
    }

    /// The collection that new entries go to: the default one,
    /// usually "Login", which is unlocked at login.
    fn collection<'a>(service: &'a SecretService<'a>) -> Result<Collection<'a>, KeyringError> {
        let collection = match service.get_default_collection() {
            Ok(collection) => collection,
            // Some services have no default; any one will do.
            Err(secret_service::Error::NoResult) => service.get_any_collection()?,
            Err(error) => return Err(error.into()),
        };

        collection.ensure_unlocked()?;

        Ok(collection)
    }

    /// Finds the entries of a name and unlocks them when needed.
    fn find<'a>(
        service: &'a SecretService<'a>,
        name: &SecretName,
    ) -> Result<Vec<Item<'a>>, KeyringError> {
        let found = service.search_items(SystemKeyring::attributes(name))?;

        if !found.locked.is_empty() {
            service.unlock_all(&found.locked.iter().collect::<Vec<_>>())?;
        }

        Ok(found.unlocked.into_iter().chain(found.locked).collect())
    }
}

impl SecretStore for SystemKeyring {
    fn store(&self, name: &SecretName, secret: &Secret) -> Result<(), KeyringError> {
        let service = SystemKeyring::connect()?;
        let collection = SystemKeyring::collection(&service)?;

        // "replace": an entry with the same attributes is
        // overwritten instead of a second one being added.
        collection.create_item(
            &format!("IO Master: {name}"),
            SystemKeyring::attributes(name),
            secret.expose().as_bytes(),
            true,
            "text/plain",
        )?;

        Ok(())
    }

    fn load(&self, name: &SecretName) -> Result<Secret, KeyringError> {
        let service = SystemKeyring::connect()?;
        let items = SystemKeyring::find(&service, name)?;

        let item = items.first().ok_or(KeyringError::NotFound)?;

        // The bytes are wiped when dropped, whatever happens next.
        let bytes = Zeroizing::new(item.get_secret()?);

        let text = std::str::from_utf8(&bytes).map_err(|_| {
            KeyringError::Failed(format!("the secret '{name}' is not text"))
        })?;

        Secret::new(text.to_owned())
    }

    fn delete(&self, name: &SecretName) -> Result<(), KeyringError> {
        let service = SystemKeyring::connect()?;
        let items = SystemKeyring::find(&service, name)?;

        if items.is_empty() {
            return Err(KeyringError::NotFound);
        }

        // Normally one entry; older tools may have left more.
        for item in &items {
            item.delete()?;
        }

        Ok(())
    }

    fn contains(&self, name: &SecretName) -> Result<bool, KeyringError> {
        let service = SystemKeyring::connect()?;

        // Searching reports locked entries too, without unlocking
        // them, so no dialog is shown.
        let found = service.search_items(SystemKeyring::attributes(name))?;

        Ok(!found.unlocked.is_empty() || !found.locked.is_empty())
    }
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "keyring_tests.rs"]
mod tests;
