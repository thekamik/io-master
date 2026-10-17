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

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, RwLock};

use tokio::sync::watch;

use crate::accounts::accounts::Account;
use crate::db::encryption::database_key::DatabaseKey;
use crate::evaluation::projects::ProjectData;
use crate::installation::lifecycle::{Opened, Running};

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
    /// For "Undo" in the drawing of the schemes: the content (tiles
    /// and links) of every test project before its last changes, the
    /// newest last; one stack per project, because a change may touch
    /// two schemes (a link between them, a moved tile). Kept in memory
    /// only, for the session.
    undo: Mutex<HashMap<i64, Vec<ProjectData>>>,
}

/// How many changes of one test project can be undone.
const UNDO_DEPTH: usize = 50;

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
            undo: Mutex::new(HashMap::new()),
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
        *lock(&self.session) = Some(Session {
            account,
            key: Arc::new(key),
        });
    }

    /// Ends the session; a reply being written and a long work are
    /// stopped.
    pub fn log_out(&self) {
        self.stop_reply();
        self.stop_work();
        lock(&self.undo).clear();
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

    /// Keeps the content of a test project before a change, for
    /// "Undo".
    pub fn remember(&self, before: ProjectData) {
        let mut undo = lock(&self.undo);
        let stack = undo.entry(before.project.id).or_default();

        stack.push(before);

        if stack.len() > UNDO_DEPTH {
            stack.remove(0);
        }
    }

    /// The content of a test project before its last change, taken
    /// off the stack; `None` when there is nothing to undo.
    pub fn take_undo(&self, project: i64) -> Option<ProjectData> {
        lock(&self.undo).get_mut(&project).and_then(Vec::pop)
    }

    /// Whether a change of the test project can be undone.
    pub fn can_undo(&self, project: i64) -> bool {
        lock(&self.undo).get(&project).is_some_and(|stack| !stack.is_empty())
    }

    /// Forgets the changes of a test project (it was deleted, or one of
    /// its schemes was, which an earlier content may still name).
    pub fn forget_undo(&self, project: i64) {
        lock(&self.undo).remove(&project);
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
