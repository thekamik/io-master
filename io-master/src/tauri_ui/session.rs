//! The session of the logged-in evaluator (`business/session.rs`), for
//! the commands: its database, on a thread for blocking work.
//!
//! ```text
//! with_user_db()     the database (the lasting state)
//! ```

use crate::db::user::database::UserDb;

use super::blocking::blocking;
use super::error::UiError;
use super::state::UiState;

/// Runs `work` on the database of the logged-in evaluator, on a
/// thread for blocking work. Fails with `NotLoggedIn` when nobody
/// is logged in: every command that touches the data of the
/// evaluator goes through here, so none can skip that check.
pub async fn with_user_db<T: Send + 'static>(
    state: &UiState,
    work: impl FnOnce(&UserDb) -> Result<T, UiError> + Send + 'static,
) -> Result<T, UiError> {
    let session = state.session()?;

    blocking(move || work(&session.db())).await
}
