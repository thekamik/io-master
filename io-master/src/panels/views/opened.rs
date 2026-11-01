//! The open database of the user logged in, as every command of a panel
//! takes it from the session: the lock of the session held for the whole
//! command, so that two commands use the database one after the other.
//!
//! ```text
//! let opened = session.lock()?;        the lock, for the command
//! let db = database(opened.as_ref())?; the database, or "nobody is logged in"
//! ```

use crate::infrastructure::storage::user_database::UserDatabase;
use crate::panels::views::error::ViewError;

/// The open database of the session `opened`; refused when nobody is
/// logged in.
///
/// Called by: the commands of `panels/views/` that use the database.
pub fn database(opened: Option<&UserDatabase>) -> Result<&UserDatabase, ViewError> {
    match opened {
        Some(database) => Ok(database),
        None => Err(ViewError::NotLoggedIn),
    }
}
