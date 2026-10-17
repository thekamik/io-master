//! The session of the logged-in evaluator (`business/session.rs`), for
//! the commands: its database and the test projects open for editing,
//! on a thread for blocking work.
//!
//! ```text
//! with_user_db()     the database (the lasting state)
//! with_workspace()   the database and the open projects
//! with_project()     the database and one open project (opened from
//!                    the database when it is not open yet)
//! ```

use crate::business::test_projects::workspace::{OpenProject, ProjectWorkspace};
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

/// Runs `work` on the database and the projects open for editing.
/// Both are locked while `work` runs (`business/session.rs`), so that
/// two commands never change one project at the same time.
pub async fn with_workspace<T: Send + 'static>(
    state: &UiState,
    work: impl FnOnce(&UserDb, &mut ProjectWorkspace) -> Result<T, UiError> + Send + 'static,
) -> Result<T, UiError> {
    let session = state.session()?;

    blocking(move || {
        let (db, mut projects) = session.projects();
        work(&db, &mut projects)
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
