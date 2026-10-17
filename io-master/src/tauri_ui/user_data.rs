//! The database of the logged-in evaluator, for the commands.

use crate::db::user::database::{self as user_db, UserDb};

use super::blocking::blocking;
use super::error::UiError;
use super::state::UiState;

/// Runs `work` on the database of the logged-in evaluator, on
/// a thread for blocking work. Fails with `NotLoggedIn` when nobody
/// is logged in: every command that touches the data of the
/// evaluator goes through here, so none can skip that check.
pub async fn with_user_db<T: Send + 'static>(
    state: &UiState,
    work: impl FnOnce(&UserDb) -> Result<T, UiError> + Send + 'static,
) -> Result<T, UiError> {
    let session = state.session()?;
    let data_dir = state.data_dir.clone();

    blocking(move || work(&user_db::open(&data_dir, session.account.id, &session.key)?)).await
}
