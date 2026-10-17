//! The logged-in evaluator (step 40): their account, their database,
//! opened once at the login, and the work open in memory.
//!
//! ```text
//! Session
//!   account       who is logged in
//!   db            the database of the evaluator, open from the login
//!                 to the logout (db/user/database.rs)
//!   projects      the test projects open for editing, with their
//!                 versions for "Undo" (test_projects/workspace.rs)
//!   evaluations   the evaluations open for work (evaluations/workspace.rs)
//! ```
//!
//! **Where the information is.** While an evaluator is logged in, the
//! state that matters is here, in the core: the page holds none of it.
//! The key of the database is not kept: it opens the database once and
//! is wiped from memory right after (the database itself, SQLCipher,
//! keeps what it needs). The logout drops the session, which closes
//! the database and forgets the work in memory.
//!
//! **One piece of work at a time.** The database and the open work are
//! each behind a lock, always taken in this order (the database first),
//! so that two commands never change one project at the same time and
//! never wait for each other in a circle. A long work (an analysis
//! with the AI) takes the session for each of its steps only, never
//! while it waits for the model.
//!
//! A lock left by a command that panicked is used as it is: a change is
//! made in memory only after the database stored it, and SQLite rolls
//! back an unfinished transaction, so nothing is left half done.

pub mod accounts;

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::business::evaluations::workspace::EvaluationWorkspace;
use crate::business::test_projects::workspace::ProjectWorkspace;
use crate::db::encryption::database_key::DatabaseKey;
use crate::db::error::DbError;
use crate::db::user::database::{self as user_db, UserDb};

use accounts::Account;

/// The logged-in evaluator.
#[derive(Debug)]
pub struct Session {
    pub account: Account,
    db: Arc<Mutex<UserDb>>,
    projects: Mutex<ProjectWorkspace>,
    evaluations: Mutex<EvaluationWorkspace>,
}

/// Locks a mutex; a lock left by a panic is used as it is (see the top
/// of this file).
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Session {
    /// Opens the session of `account`, whose database in `data_dir`
    /// opens with `key` (checked by the login, `accounts.rs`). The key
    /// is dropped, and so wiped, when this function returns.
    pub fn open(data_dir: &Path, account: Account, key: DatabaseKey) -> Result<Session, DbError> {
        let db = user_db::open(data_dir, account.id, &key)?;

        Ok(Session {
            account,
            db: Arc::new(Mutex::new(db)),
            projects: Mutex::new(ProjectWorkspace::default()),
            evaluations: Mutex::new(EvaluationWorkspace::default()),
        })
    }

    /// The database of the evaluator, for one piece of work.
    pub fn db(&self) -> MutexGuard<'_, UserDb> {
        lock(&self.db)
    }

    /// The database as a shared handle, for `ai/api/`, which writes the
    /// journal of the AI after a reply (`completions::ask`).
    pub fn db_handle(&self) -> Arc<Mutex<UserDb>> {
        self.db.clone()
    }

    /// The database and the test projects open for editing, for one
    /// piece of work (locked in this order).
    pub fn projects(&self) -> (MutexGuard<'_, UserDb>, MutexGuard<'_, ProjectWorkspace>) {
        let db = lock(&self.db);

        (db, lock(&self.projects))
    }

    /// The database and the evaluations open for work, for one piece
    /// of work (locked in this order).
    pub fn evaluations(&self) -> (MutexGuard<'_, UserDb>, MutexGuard<'_, EvaluationWorkspace>) {
        let db = lock(&self.db);

        (db, lock(&self.evaluations))
    }
}
