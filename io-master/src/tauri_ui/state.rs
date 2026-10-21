//! The state of the desktop interface, kept in Rust and never in
//! the page: the installation, the session of the logged-in evaluator
//! (`installation/session.rs`: the account, the open database and the work
//! in memory), and the stop signals of the work that is running.
//!
//! The page (`ui/`) holds no secret and no state that matters: it
//! asks through the commands (`commands.rs`) and shows what it gets
//! back. Reloading the page loses nothing; ending the program ends
//! the session and wipes the key of the database from memory.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, RwLock};

use tokio::sync::watch;

use crate::installation::accounts::Account;
use crate::installation::lifecycle::{Opened, Running};
use crate::installation::session::Session;
use crate::model::installation::Role;

use super::error::UiError;

/// Everything the commands reach. One for the whole program.
#[derive(Debug)]
pub struct UiState {
    /// The folder of the installation.
    pub data_dir: PathBuf,
    /// The running installation; `None` until the first start is
    /// done. Written once, by the setup.
    running: RwLock<Option<Arc<Running>>>,
    /// The session of the logged-in evaluator; `None` while nobody is
    /// logged in. Shared with the work that runs at the moment.
    session: Mutex<Option<Arc<Session>>>,
    /// The stop signal of the reply that is being written, if one
    /// is: sending `true` stops it (the "Stop" button).
    reply_stop: Mutex<Option<watch::Sender<bool>>>,
    /// The stop signal of a long work (loading a standard, an
    /// analysis), the same way.
    work_stop: Mutex<Option<watch::Sender<bool>>>,
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
        }
    }

    /// The running installation, or `None` before the first start
    /// is done.
    pub fn running(&self) -> Option<Arc<Running>> {
        self.running.read().unwrap_or_else(|poisoned| poisoned.into_inner()).clone()
    }

    /// The role of the installation, or `None` before the first start
    /// is done.
    pub fn role(&self) -> Option<Role> {
        Some(self.running()?.installation.role)
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

    /// The session of the logged-in evaluator, or the error that
    /// nobody is logged in. Every command that reads or changes the
    /// data of the evaluator starts with this check.
    pub fn session(&self) -> Result<Arc<Session>, UiError> {
        lock(&self.session).clone().ok_or(UiError::NotLoggedIn)
    }

    /// Whether somebody is logged in, and who.
    pub fn current_user(&self) -> Option<Account> {
        lock(&self.session).as_ref().map(|session| session.account.clone())
    }

    /// Starts the session of a logged-in evaluator, replacing an
    /// earlier one.
    pub fn log_in(&self, session: Session) {
        *lock(&self.session) = Some(Arc::new(session));
    }

    /// Ends the session; a reply being written and a long work are
    /// stopped. The database closes and the work in memory is forgotten
    /// when the last holder of the session (a work still finishing)
    /// lets it go.
    pub fn log_out(&self) {
        self.stop_reply();
        self.stop_work();
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
