//! The session: what every command of the window shares (user,
//! 2026-10-28): the data folder of the program, and, while the user is
//! logged in, the user and their database, open. The database is opened
//! once at the login (`panels/logic/program_start/access.rs`) and kept here until the logout
//! or the end of the program; the key that opened it is already wiped,
//! SQLCipher keeps its own inside the connection.
//!
//! ```text
//! the window ──► Session (one, kept by Tauri) ──► Mutex ──► None              logged out
//!                                                        └► Some(Opened)      logged in:
//!                                                              user, database
//! ```
//!
//! This is the only global state of the program (rule 6 of CLAUDE.md): the
//! commands run each on its own, and the session is how they find the open
//! database. The lock makes two commands use it one after the other.
//!
//! Besides, the session knows whether a long work of the AI runs ("Fill
//! with AI", user 2026-10-29; a question of a chat, 2026-10-30), so that
//! only one runs at a time, and whether
//! the user asked it to stop: the work holds the lock only for its short
//! steps, and "Stop" is another command.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};

use crate::db::database::Database;
use crate::model::user::user_data::User;

/// The user logged in and their open database.
pub struct Opened {
    pub user: User,
    pub database: Database,
}

/// The session of the window.
pub struct Session {
    data_folder: PathBuf,
    opened: Mutex<Option<Opened>>,
    /// A long work of the AI runs.
    working: AtomicBool,
    /// The user asked the long work to stop.
    stop: AtomicBool,
}

/// A long work of the AI that runs; when it is dropped, however the work
/// ends, the session knows it no more runs.
pub struct Work<'session>(&'session Session);

impl Drop for Work<'_> {
    fn drop(&mut self) {
        self.0.working.store(false, Ordering::SeqCst);
    }
}

/// The lock of the session could not be taken: a command failed while it
/// held it. The window asks the user to start the program again.
#[derive(Debug)]
pub struct SessionBroken;

impl Session {
    /// A session of the program whose data is in `data_folder`, nobody
    /// logged in.
    ///
    /// Called by: `main.rs` (the start of the window).
    pub fn new(data_folder: PathBuf) -> Session {
        Session { data_folder, opened: Mutex::new(None), working: AtomicBool::new(false), stop: AtomicBool::new(false) }
    }

    /// The data folder of the program.
    ///
    /// Called by: the commands of `panels/views/program_start/`.
    pub fn data_folder(&self) -> &Path {
        &self.data_folder
    }

    /// The opened session, locked for one command: `None` when nobody is
    /// logged in.
    ///
    /// Called by: [`Session::log_in`], [`Session::log_out`], the commands of
    /// `panels/views/`.
    pub fn lock(&self) -> Result<MutexGuard<'_, Option<Opened>>, SessionBroken> {
        self.opened.lock().map_err(|_| SessionBroken)
    }

    /// Keeps `user` and their open `database` as the session; a session
    /// open before is closed.
    ///
    /// Called by: `panels/views/program_start/installation.rs`, `login.rs`.
    /// Calls: [`Session::lock`].
    pub fn log_in(&self, user: User, database: Database) -> Result<(), SessionBroken> {
        *self.lock()? = Some(Opened { user, database });

        Ok(())
    }

    /// Begins a long work of the AI: `None` when one runs already. The
    /// request to stop of an earlier work is forgotten.
    ///
    /// Called by: `panels/views/evaluation_plan/fill.rs` (`plan_fill`),
    /// `panels/views/chat/chats.rs` (`chat_ask`).
    pub fn begin_work(&self) -> Option<Work<'_>> {
        if self.working.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
            return None;
        }

        self.stop.store(false, Ordering::SeqCst);

        Some(Work(self))
    }

    /// Asks the long work that runs to stop after its present step.
    ///
    /// Called by: `panels/views/evaluation_plan/fill.rs` (`plan_fill_stop`),
    /// `panels/views/chat/chats.rs` (`chat_stop`).
    pub fn request_stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }

    /// Whether the user asked the long work to stop.
    ///
    /// Called by: `panels/views/evaluation_plan/fill.rs` (before each step),
    /// `panels/views/chat/chats.rs` (while an answer streams).
    pub fn stop_requested(&self) -> bool {
        self.stop.load(Ordering::SeqCst)
    }

    /// Ends the session: the database is closed, nobody is logged in.
    ///
    /// Called by: `panels/views/program_start/login.rs` (`login_log_out`); `main.rs`
    /// (the end of the program, however it ends).
    /// Calls: [`Session::lock`].
    pub fn log_out(&self) -> Result<(), SessionBroken> {
        *self.lock()? = None;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    //! Tests of the session: logged in, logged out.

    use super::*;

    use crate::test_support::database::database;

    #[test]
    fn a_user_logs_in_and_out() {
        let (directory, db) = database();
        let session = Session::new(directory.path().to_owned());
        let user = User { id: 1, name: "Kamil".to_owned(), created: 1 };

        assert!(session.lock().unwrap().is_none());

        session.log_in(user.clone(), db).unwrap();

        assert_eq!(session.lock().unwrap().as_ref().map(|opened| opened.user.clone()), Some(user));

        session.log_out().unwrap();

        assert!(session.lock().unwrap().is_none());
        assert_eq!(session.data_folder(), directory.path());
    }

    #[test]
    fn one_long_work_runs_at_a_time_and_can_be_asked_to_stop() {
        let session = Session::new(std::path::PathBuf::from("/nowhere"));
        let work = session.begin_work();

        assert!(work.is_some() && session.begin_work().is_none());

        session.request_stop();
        assert!(session.stop_requested());

        drop(work);

        assert!(session.begin_work().is_some() && !session.stop_requested());
    }
}
