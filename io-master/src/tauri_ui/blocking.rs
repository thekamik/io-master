//! Blocking work of the commands, kept off the threads that keep
//! the interface answering.

use super::error::UiError;

/// Runs blocking work (Argon2id, the databases, the scan) on the
/// threads that tokio keeps for it, so that the interface keeps
/// answering meanwhile. A thread that panicked is reported as
/// a failure, never passed on as a panic.
pub async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, UiError> + Send + 'static,
) -> Result<T, UiError> {
    tokio::task::spawn_blocking(work)
        .await
        .unwrap_or_else(|_| Err(UiError::Failed("the work was interrupted".to_owned())))
}
