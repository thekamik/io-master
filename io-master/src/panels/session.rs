//! The session: what every command of the window shares (user,
//! 2026-10-28): the data folder of the program, and, while the user is
//! logged in, the user and their database, open. The database is opened
//! once at the login (`objects/user/user_data/user_data_rules.rs`) and kept here until the logout
//! or the end of the program; the key that opened it is already wiped,
//! `SQLCipher` keeps its own inside the connection.
//!
//! ```text
//! the window ──► Session (one, kept by Tauri) ──► user      Mutex ──► None / Some(User)
//!                                             ──► database  Mutex ──► None / Some(UserDatabase)
//!                                             ──► progress  Mutex ──► Progress (progress_model.rs)
//! ```
//!
//! This is the only global state of the program (rule 6 of CLAUDE.md): the
//! commands run each on its own, and the session is how they find the open
//! database. The lock makes two commands use it one after the other.
//!
//! Besides, the session holds the progress of the long work of the AI
//! ("Fill with AI", the assessment of an evaluation, a question of a chat):
//! only one runs at a time, the user may ask it to stop ("Stop" is another
//! command), and the page asks for its progress, also when a panel is
//! opened again (user, 2026-11-01). The work holds the lock of the database
//! only for its short steps.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::session::progress::progress_model::Progress;
use crate::objects::user::user_data::user_data_model::User;

/// The session of the window.
pub struct Session {
    data_folder: PathBuf,
    /// The user logged in.
    user: Mutex<Option<User>>,
    /// Their open database.
    database: Mutex<Option<UserDatabase>>,
    /// The progress of the long work of the AI.
    progress: Mutex<Progress>,
}

/// A long work of the AI that runs; when it is dropped, however the work
/// ends, the session knows it no more runs.
pub struct Work<'session>(&'session Session);

impl Drop for Work<'_> {
    fn drop(&mut self) {
        // A broken lock cannot be mended here; the next work is then
        // refused by begin_work, which reads the same lock.
        if let Ok(mut progress) = self.0.progress.lock() {
            progress.running = false;
        }
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
    /// Input: `data_folder`, the data folder of the program
    /// (`~/.local/share/io-master`).
    /// Output: the session.
    ///
    /// Called by: `main.rs`: `tauri_startup`.
    pub fn new(data_folder: PathBuf) -> Session {
        Session {
            data_folder,
            user: Mutex::new(None),
            database: Mutex::new(None),
            progress: Mutex::new(Progress::default()),
        }
    }

    /// The data folder of the program.
    ///
    /// Called by: `panels/views/account/credentials.rs`: `account_change_password`,
    /// `account_new_recovery_code`; `panels/views/program_start/installation.rs`:
    /// `installation_create`; `panels/views/program_start/login.rs`: `login_get`,
    /// `login_log_in`, `login_recover`.
    pub fn data_folder(&self) -> &Path {
        &self.data_folder
    }

    /// The open database, locked for one command: `None` when nobody is
    /// logged in.
    ///
    /// Called by: [`Session::log_in`], [`Session::log_out`];
    /// `panels/views/agents/agent.rs`: `agent_list`, `agent_create`, `agent_get`, `agent_save`,
    /// `agent_knowledge_test`; `panels/views/agents/journal.rs`: `agent_journal_list`,
    /// `agent_journal_get`; `panels/views/agents/knowledge.rs`: `knowledge_base_list`,
    /// `knowledge_base_create`, `knowledge_base_save`, `knowledge_source_list`, `knowledge_source_add`,
    /// `knowledge_source_replace`, `knowledge_source_rename`, `knowledge_search`;
    /// `panels/views/chat/chats.rs`: `chat_list`, `chat_create`, `chat_get`, `chat_rename`,
    /// `prepared_question`, `written`; `panels/views/evaluation/evaluations.rs`: `evaluation_list`,
    /// `evaluation_create`, `evaluation_get`, `evaluation_set_agent`;
    /// `panels/views/evaluation/inputs.rs`: `evaluation_document_give`, `evaluation_proof_give`;
    /// `panels/views/evaluation/run.rs`: `evaluation_assess_preview`, `evaluation_assess`;
    /// `panels/views/evaluation/work_units.rs`: `evaluation_unit_list`, `evaluation_unit_get`,
    /// `evaluation_unit_save`; `panels/views/evaluation_plan/fill.rs`: `fill`, `plan_fill_preview`;
    /// `panels/views/evaluation_plan/plans.rs`: `plan_list`, `plan_create`, `plan_get`, `plan_save`,
    /// `plan_add_document`; `panels/views/evaluation_plan/standards.rs`: `standard_list`,
    /// `standard_import`; `panels/views/evaluation_plan/work_units.rs`: `work_unit_list`,
    /// `work_unit_controls`, `work_unit_add`, `work_unit_add_all`, `work_unit_remove`, `work_unit_get`,
    /// `work_unit_save`, `work_unit_add_proof`; `panels/views/settings/integrations.rs`:
    /// `integration_list`, `integration_create`, `integration_save`, `integration_activate`,
    /// `integration_set_key`, `integration_check`.
    pub fn lock(&self) -> Result<MutexGuard<'_, Option<UserDatabase>>, SessionBroken> {
        self.database.lock().map_err(|_| SessionBroken)
    }

    /// The name of the user logged in; `None` when nobody is.
    ///
    /// Called by: `panels/views/program_start/login.rs`: `login_get`.
    pub fn user_name(&self) -> Result<Option<String>, SessionBroken> {
        let user = self.user.lock().map_err(|_| SessionBroken)?;

        Ok(user.as_ref().map(|user| user.name.clone()))
    }

    /// The id of the user logged in (`Some(1)`); `None` when nobody is.
    ///
    /// Called by: `panels/views/account/credentials.rs`: `account_change_password`,
    /// `account_new_recovery_code`.
    pub fn user_id(&self) -> Result<Option<i64>, SessionBroken> {
        let user = self.user.lock().map_err(|_| SessionBroken)?;

        Ok(user.as_ref().map(|user| user.id))
    }

    /// Keeps `user` and their open `database` as the session; a session
    /// open before is closed.
    ///
    /// Input: `user`, the user whose password or code opened `database`;
    /// `database`, their database, open.
    /// Output: nothing; `SessionBroken` when a lock of the session is broken.
    ///
    /// Called by: `panels/views/program_start/installation.rs`: `installation_create`;
    /// `panels/views/program_start/login.rs`: `login_log_in`, `login_recover`.
    /// Calls: [`Session::lock`].
    pub fn log_in(&self, user: User, database: UserDatabase) -> Result<(), SessionBroken> {
        // The database first, then the user, the order of log_out reversed:
        // the session never names a user without their database.
        *self.lock()? = Some(database);
        *self.user.lock().map_err(|_| SessionBroken)? = Some(user);

        Ok(())
    }

    /// The locks of the open database and of the progress, for a long work
    /// of the AI that takes them for its short steps (`objects/session/progress/progress_ai/lock.rs`).
    ///
    /// Called by: `panels/views/evaluation/run.rs`: `evaluation_assess`;
    /// `panels/views/evaluation_plan/fill.rs`: `fill`.
    pub fn locks(&self) -> (&Mutex<Option<UserDatabase>>, &Mutex<Progress>) {
        (&self.database, &self.progress)
    }

    /// The progress of the long work of the AI, locked: for the work that
    /// writes its steps into it and for the page that reads it.
    ///
    /// Called by: [`Session::begin_work`], [`Session::request_stop`],
    /// [`Session::stop_requested`], [`Session::is_working`];
    /// `panels/views/progress.rs`: `progress_of`.
    pub fn progress(&self) -> Result<MutexGuard<'_, Progress>, SessionBroken> {
        self.progress.lock().map_err(|_| SessionBroken)
    }

    /// Begins a long work of the AI on the plan or evaluation
    /// `evaluation_plan`, over `total` work units. The progress of the work
    /// before is forgotten.
    ///
    /// Input: `evaluation_plan`, the id of the plan or evaluation, `None` for a
    /// chat; `total`, how many steps the work has.
    /// Output: the work, which ends the running when dropped; `None` when one
    /// runs already; `SessionBroken` when the lock of the progress is broken.
    ///
    /// Called by: `panels/views/evaluation_plan/fill.rs`: `fill`;
    /// `panels/views/evaluation/run.rs`: `evaluation_assess`;
    /// `panels/views/chat/chats.rs`: `chat_ask`.
    /// Calls: [`Session::progress`].
    pub fn begin_work(&self, evaluation_plan: Option<i64>, total: usize) -> Result<Option<Work<'_>>, SessionBroken> {
        let mut progress = self.progress()?;

        if progress.running {
            return Ok(None);
        }

        *progress = Progress { running: true, evaluation_plan, total, ..Progress::default() };

        Ok(Some(Work(self)))
    }

    /// Whether a long work of the AI runs.
    ///
    /// Called by: `panels/views/evaluation/work_units.rs`: `evaluation_unit_save` (the
    /// evaluator and the AI never write at once).
    /// Calls: [`Session::progress`].
    pub fn is_working(&self) -> Result<bool, SessionBroken> {
        Ok(self.progress()?.running)
    }

    /// Asks the long work that runs to stop after its present step.
    ///
    /// Called by: `panels/views/evaluation_plan/fill.rs`: `plan_fill_stop`;
    /// `panels/views/evaluation/run.rs`: `evaluation_assess_stop`;
    /// `panels/views/chat/chats.rs`: `chat_stop`.
    /// Calls: [`Session::progress`].
    pub fn request_stop(&self) -> Result<(), SessionBroken> {
        self.progress()?.stop = true;

        Ok(())
    }

    /// Whether the user asked the long work to stop.
    ///
    /// Called by: `panels/views/chat/chats.rs`: `forwarded`, while an answer
    /// streams. A filling and an assessment read the same field themselves,
    /// before each step, through the lock of the progress.
    /// Calls: [`Session::progress`].
    pub fn stop_requested(&self) -> Result<bool, SessionBroken> {
        Ok(self.progress()?.stop)
    }

    /// Ends the session: the database is closed, nobody is logged in.
    ///
    /// Output: nothing; `SessionBroken` when a lock of the session is broken.
    ///
    /// Called by: `panels/views/program_start/login.rs`: `login_log_out`; `main.rs`:
    /// `run_window` (the end of the program, however it ends).
    /// Calls: [`Session::lock`].
    pub fn log_out(&self) -> Result<(), SessionBroken> {
        *self.user.lock().map_err(|_| SessionBroken)? = None;
        *self.lock()? = None;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    //! Tests of the session: logged in, logged out; one long work at a time,
    //! its progress.

    use super::*;

    use crate::test_support::database::database;

    #[test]
    fn a_user_logs_in_and_out() {
        let (directory, db) = database();
        let session = Session::new(directory.path().to_owned());
        let user = User { id: 1, name: "Kamil".to_owned(), created: 1 };

        assert!(session.lock().unwrap().is_none());

        session.log_in(user, db).unwrap();

        assert!(session.lock().unwrap().is_some());
        assert_eq!(session.user_name().unwrap().as_deref(), Some("Kamil"));

        session.log_out().unwrap();

        assert!(session.lock().unwrap().is_none() && session.user_name().unwrap().is_none());
        assert_eq!(session.data_folder(), directory.path());
    }

    #[test]
    fn one_long_work_runs_at_a_time_and_can_be_asked_to_stop() {
        let session = Session::new(std::path::PathBuf::from("/nowhere"));
        let work = session.begin_work(Some(4), 10).unwrap();

        assert!(work.is_some() && session.begin_work(None, 0).unwrap().is_none());
        assert!(session.is_working().unwrap());

        session.progress().unwrap().assessed = 3;
        session.request_stop().unwrap();
        assert!(session.stop_requested().unwrap());

        drop(work);

        // The progress of the work that ended stays for the page.
        let ended = session.progress().unwrap().clone();

        assert_eq!((ended.running, ended.evaluation_plan, ended.total, ended.assessed), (false, Some(4), 10, 3));

        let next = session.begin_work(None, 0).unwrap();

        assert!(next.is_some() && !session.stop_requested().unwrap());
        assert_eq!(session.progress().unwrap().evaluation_plan, None);
    }
}
