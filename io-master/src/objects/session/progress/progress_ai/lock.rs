//! The locks of the session as a long work of the AI takes them (user,
//! 2026-11-01): the open database of the user, held only for the short
//! steps of the work and released while the AI answers; the progress of the
//! work, written after each step. The session owns both
//! (`panels/session.rs`); a work of `ai/` gets them as parameters and never
//! sees the session.
//!
//! ```text
//! Session ──locks()──► (&Mutex<Option<UserDatabase>>, &Mutex<Progress>) ──► a loop of ai/models/evaluation/
//!                          lock_database ──► the guard ──► open_database ──► &UserDatabase
//!                          lock_progress ──► the guard ──► record_step, stop
//! ```

use std::sync::{Mutex, MutexGuard};

use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::ai_error::AiError;
use crate::objects::session::progress::progress_model::Progress;

/// The open database of the session, locked for one step of a long work.
///
/// Input:
/// - `database`: the lock of the open database of the session.
///
/// Output: the guard; `None` in it when nobody is logged in (see
/// [`open_database`]). An error when a command failed while it held the
/// lock: the session is broken.
///
/// Called by: `objects/evaluation/work_unit/work_unit_ai/assessment.rs`,
/// `work_unit_ai/filling.rs`: their steps under the lock.
pub fn lock_database(database: &Mutex<Option<UserDatabase>>) -> Result<MutexGuard<'_, Option<UserDatabase>>, AiError> {
    database.lock().map_err(|_| AiError::Refused("the session is broken: start the program again".to_owned()))
}

/// The open database in the guard `opened`; refused when the user logged
/// out while the work ran.
///
/// Called by: `objects/evaluation/work_unit/work_unit_ai/assessment.rs`, `filling.rs`.
pub fn open_database(opened: &Option<UserDatabase>) -> Result<&UserDatabase, AiError> {
    match opened {
        Some(database) => Ok(database),
        None => Err(AiError::Refused("nobody is logged in: the work ends".to_owned())),
    }
}

/// The progress of the long work, locked for a moment.
///
/// Output: the guard. An error when the session is broken.
///
/// Called by: `objects/evaluation/evaluation_plan/evaluation_plan_ai.rs`: `assess_evaluation`,
/// `fill_plan`.
pub fn lock_progress(progress: &Mutex<Progress>) -> Result<MutexGuard<'_, Progress>, AiError> {
    progress.lock().map_err(|_| AiError::Refused("the session is broken: start the program again".to_owned()))
}
