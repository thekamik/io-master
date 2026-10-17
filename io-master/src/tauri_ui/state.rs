//! The state of the desktop interface, kept in Rust and never in
//! the page: the installation, the logged-in evaluator with the key
//! of their database, and the reply that is being written. (The AI
//! connections are data of the evaluator, kept in their database:
//! `db/ai/connection/`.)
//!
//! The page (`ui/`) holds no secret and no state that matters: it
//! asks through the commands (`commands.rs`) and shows what it gets
//! back. Reloading the page loses nothing; ending the program ends
//! the session and wipes the key of the database from memory.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, RwLock};

use tokio::sync::watch;

use crate::business::installation::lifecycle::{Opened, Running};
use crate::business::session::accounts::Account;
use crate::business::test_projects::workspace::Workspace;
use crate::db::encryption::database_key::DatabaseKey;

use super::error::UiError;

/// The logged-in evaluator.
#[derive(Debug, Clone)]
pub struct Session {
    pub account: Account,
    /// The key of the database of the account, opened by the
    /// password at login. Shared with the work that runs at the
    /// moment; wiped when the last holder drops it.
    pub key: Arc<DatabaseKey>,
}

/// Everything the commands reach. One for the whole program.
#[derive(Debug)]
pub struct UiState {
    /// The folder of the installation.
    pub data_dir: PathBuf,
    /// The running installation; `None` until the first start is
    /// done. Written once, by the setup.
    running: RwLock<Option<Arc<Running>>>,
    /// The logged-in evaluator; `None` while nobody is logged in.
    session: Mutex<Option<Session>>,
    /// The stop signal of the reply that is being written, if one
    /// is: sending `true` stops it (the "Stop" button).
    reply_stop: Mutex<Option<watch::Sender<bool>>>,
    /// The stop signal of a long work (loading a standard, an
    /// analysis), the same way.
    work_stop: Mutex<Option<watch::Sender<bool>>>,
    /// The test projects open for editing, with their versions for
    /// "Undo" (`business/test_projects/workspace.rs`): the objects in memory of
    /// the session. Shared with the work on the database, which runs
    /// on a thread of its own (`user_data.rs`, `with_project`).
    workspace: Arc<Mutex<Workspace>>,
}

/// Locks a mutex. A panic in another command while it held the lock
/// cannot leave the value half-written (every change is a single
/// assignment), so a poisoned lock is used as it is.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl UiState {
    /// The state at the start of the program.
    pub fn new(data_dir: PathBuf, opened: Opened) -> UiState {
        let running = match opened {
            Opened::FirstRun => None,
            Opened::Installed(running) => Some(Arc::new(running)),
        };

        UiState {
            data_dir,
            running: RwLock::new(running),
            session: Mutex::new(None),
            reply_stop: Mutex::new(None),
            work_stop: Mutex::new(None),
            workspace: Arc::new(Mutex::new(Workspace::default())),
        }
    }

    /// The running installation, or `None` before the first start
    /// is done.
    pub fn running(&self) -> Option<Arc<Running>> {
        self.running.read().unwrap_or_else(|poisoned| poisoned.into_inner()).clone()
    }

    /// The running installation, or the error that nothing is
    /// installed yet.
    pub fn installed(&self) -> Result<Arc<Running>, UiError> {
        self.running().ok_or(UiError::NotInstalled)
    }

    /// Records the installation made by the setup.
    pub fn install(&self, running: Running) {
        *self.running.write().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(Arc::new(running));
    }

    /// The logged-in evaluator, or the error that nobody is logged
    /// in. Every command that reads or changes the data of the
    /// evaluator starts with this check.
    pub fn session(&self) -> Result<Session, UiError> {
        lock(&self.session).clone().ok_or(UiError::NotLoggedIn)
    }

    /// Whether somebody is logged in, and who.
    pub fn current_user(&self) -> Option<Account> {
        lock(&self.session).as_ref().map(|session| session.account.clone())
    }

    /// Starts the session of a logged-in evaluator, replacing an
    /// earlier one.
    pub fn log_in(&self, account: Account, key: DatabaseKey) {
        *lock(&self.session) = Some(Session { account, key: Arc::new(key) });
    }

    /// Ends the session; a reply being written and a long work are
    /// stopped.
    pub fn log_out(&self) {
        self.stop_reply();
        self.stop_work();
        lock(&self.workspace).clear();
        *lock(&self.session) = None;
    }

    /// Registers the reply that starts now and returns its stop
    /// signal. A reply still being written is stopped first: one
    /// reply at a time.
    pub fn start_reply(&self) -> watch::Receiver<bool> {
        let (sender, receiver) = watch::channel(false);

        if let Some(earlier) = lock(&self.reply_stop).replace(sender) {
            let _ = earlier.send(true);
        }

        receiver
    }

    /// Stops the reply being written, if there is one.
    pub fn stop_reply(&self) {
        if let Some(stop) = lock(&self.reply_stop).take() {
            let _ = stop.send(true);
        }
    }

    /// Registers a long work that starts now and returns its stop
    /// signal; a long work still running is stopped first.
    pub fn start_work(&self) -> watch::Receiver<bool> {
        let (sender, receiver) = watch::channel(false);

        if let Some(earlier) = lock(&self.work_stop).replace(sender) {
            let _ = earlier.send(true);
        }

        receiver
    }

    /// The test projects open for editing, to be used on the thread
    /// of the database (`user_data.rs`, `with_project`).
    pub fn workspace(&self) -> Arc<Mutex<Workspace>> {
        self.workspace.clone()
    }

    /// Stops the long work, if there is one.
    pub fn stop_work(&self) {
        if let Some(stop) = lock(&self.work_stop).take() {
            let _ = stop.send(true);
        }
    }
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
