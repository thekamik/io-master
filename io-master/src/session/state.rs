//! The session: what every command of the window shares (user,
//! 2026-10-28): the data folder of the program, and, while the user is
//! logged in, the user and their database, open. The database is opened
//! once at the login (`account/access.rs`) and kept here until the logout
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

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use crate::db::database::Database;
use crate::model::user::user_data::User;

/// The user logged in and their open database.
pub struct Opened {
    pub user: User,
    #[allow(dead_code, reason = "read by the views that come next (2026-10-28)")]
    pub database: Database,
}

/// The session of the window.
pub struct Session {
    data_folder: PathBuf,
    opened: Mutex<Option<Opened>>,
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
        Session { data_folder, opened: Mutex::new(None) }
    }

    /// The data folder of the program.
    ///
    /// Called by: the commands of `views/program_start/`.
    pub fn data_folder(&self) -> &Path {
        &self.data_folder
    }

    /// The opened session, locked for one command: `None` when nobody is
    /// logged in.
    ///
    /// Called by: [`Session::log_in`], [`Session::log_out`], the commands of
    /// `views/`.
    pub fn lock(&self) -> Result<MutexGuard<'_, Option<Opened>>, SessionBroken> {
        self.opened.lock().map_err(|_| SessionBroken)
    }

    /// Keeps `user` and their open `database` as the session; a session
    /// open before is closed.
    ///
    /// Called by: `views/program_start/installation.rs`, `login.rs`.
    /// Calls: [`Session::lock`].
    pub fn log_in(&self, user: User, database: Database) -> Result<(), SessionBroken> {
        *self.lock()? = Some(Opened { user, database });

        Ok(())
    }

    /// Ends the session: the database is closed, nobody is logged in.
    ///
    /// Called by: `views/program_start/login.rs` (`login_log_out`); `main.rs`
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
}
