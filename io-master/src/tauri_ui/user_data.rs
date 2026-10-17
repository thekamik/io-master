//! The database of the logged-in evaluator, and the test projects
//! open for editing, for the commands.
//!
//! ```text
//! with_user_db()     the database (the lasting state)
//! with_workspace()   the database and the open projects
//! with_project()     the database and one open project (opened from
//!                    the database when it is not open yet)
//! ```

use std::sync::PoisonError;

use crate::db::user::database::{self as user_db, UserDb};
use crate::test_projects::workspace::{OpenProject, Workspace};

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

/// Runs `work` on the database and the projects open for editing.
/// The open projects are locked while `work` runs, so that two
/// commands never change one project at the same time. A panic of
/// another command while it held the lock cannot leave a project half
/// changed in memory (a change is made in memory only after the
/// database stored it), so a poisoned lock is used as it is.
pub async fn with_workspace<T: Send + 'static>(
    state: &UiState,
    work: impl FnOnce(&UserDb, &mut Workspace) -> Result<T, UiError> + Send + 'static,
) -> Result<T, UiError> {
    let workspace = state.workspace();

    with_user_db(state, move |db| {
        let mut workspace = workspace.lock().unwrap_or_else(PoisonError::into_inner);
        work(db, &mut workspace)
    })
    .await
}

/// Runs `work` on the database and the open test project `project`.
pub async fn with_project<T: Send + 'static>(
    state: &UiState,
    project: i64,
    work: impl FnOnce(&UserDb, &mut OpenProject) -> Result<T, UiError> + Send + 'static,
) -> Result<T, UiError> {
    with_workspace(state, move |db, workspace| work(db, workspace.open(db, project)?)).await
}
