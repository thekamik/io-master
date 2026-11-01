//! The assessment of one work unit of an evaluation by the AI (user,
//! 2026-10-30), as a step of the assessment of the evaluation
//! (`evaluation_plan_ai.rs`: `assess_evaluation`).
//!
//! ```text
//! assess_work_unit ──► prepared_question   (the lock of the database held)  the question with the documents
//!                                           whole, or their passages when they do not fit
//!                  ──► send_whole          (released)                        the answer of the AI
//!                  ──► written             (held)                            record (the journal of the agent),
//!                                           read_assessment, apply_assessment ──► the verdict and the notes
//! ```
//!
//! A work unit the AI cannot assess (a question refused, an answer that is
//! no assessment) gives no verdict and the reason; only a failure of the
//! session or of the database is an error.

use std::sync::Mutex;

use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::ai::integration::integration_ai::question::{Prepared, prepare};
use crate::objects::ai::integration::integration_ai::sending::{Sent, send_whole};
use crate::objects::ai::journal::journal_ai::record;
use crate::objects::ai_error::AiError;
use crate::objects::evaluation::work_unit::work_unit_ai::assessment_answer::{apply_assessment, read_assessment};
use crate::objects::evaluation::work_unit::work_unit_ai::assessment_question::assessment_question;
use crate::objects::evaluation::work_unit::work_unit_model::Verdict;
use crate::objects::evaluation::work_unit::work_unit_store::read_work_unit;
use crate::objects::session::progress::progress_ai::lock::{lock_database, open_database};

/// The question of the work unit `unit`, prepared: with its documents whole,
/// or with their passages when they do not fit into the context of the model.
///
/// Called by: [`prepared_question`].
/// Calls: `assessment_question.rs`: `assessment_question`; `objects/ai/integration/integration_ai/question.rs`:
/// `prepare`.
pub(in crate::objects::evaluation) fn prepared_assessment(
    db: &UserDatabase,
    evaluation: i64,
    unit: i64,
) -> Result<Prepared, AiError> {
    match prepare(db, &assessment_question(db, evaluation, unit, true)?) {
        Ok(prepared) => Ok(prepared),
        Err(AiError::TooLong { .. }) => Ok(prepare(db, &assessment_question(db, evaluation, unit, false)?)?),
        Err(error) => Err(error),
    }
}

/// Reads the answer `answer` and writes it into the work unit `unit`; gives
/// the notes of what was not taken.
///
/// Called by: [`written`].
/// Calls: `assessment_answer.rs`: `read_assessment`, `apply_assessment`.
pub(in crate::objects::evaluation) fn assessed(
    db: &UserDatabase,
    evaluation: i64,
    unit: i64,
    answer: &str,
) -> Result<Vec<String>, AiError> {
    apply_assessment(db, evaluation, unit, &read_assessment(answer)?)
}

/// The label of the work unit `unit` and its question, prepared; the reason
/// instead of the question when it is refused (no room in the context, no
/// key).
///
/// Called by: [`assess_work_unit`].
/// Calls: `objects/session/progress/progress_ai/lock.rs`: `lock_database`, `open_database`;
/// `objects/evaluation/work_unit/work_unit_store.rs`: `read_work_unit`; [`prepared_assessment`].
fn prepared_question(
    database: &Mutex<Option<UserDatabase>>,
    evaluation: i64,
    unit: i64,
) -> Result<(String, Result<Prepared, String>), AiError> {
    let opened = lock_database(database)?;
    let db = open_database(opened.as_ref())?;
    let label = read_work_unit(db, unit)?.label;

    Ok((label, prepared_assessment(db, evaluation, unit).map_err(|error| error.to_string())))
}

/// Writes the answer into the journal of the agent, then into the work unit
/// `unit`; gives its verdict and the notes, or no verdict and why.
///
/// Called by: [`assess_work_unit`].
/// Calls: `objects/session/progress/progress_ai/lock.rs`: `lock_database`, `open_database`;
/// `objects/ai/journal/journal_ai.rs`: `record`; [`assessed`]; `read_work_unit`.
fn written(
    database: &Mutex<Option<UserDatabase>>,
    step: (i64, i64),
    prepared: Prepared,
    sent: Sent,
) -> Result<(Option<Verdict>, Vec<String>), AiError> {
    let (evaluation, unit) = step;
    let opened = lock_database(database)?;
    let db = open_database(opened.as_ref())?;
    let answer = record(db, prepared, sent)?;

    if let Some(error) = answer.error {
        return Ok((None, vec![error.to_string()]));
    }

    Ok(match assessed(db, evaluation, unit, &answer.text) {
        Ok(notes) => (read_work_unit(db, unit)?.verdict, notes),
        Err(error) => (None, vec![error.to_string()]),
    })
}

/// Assesses the work unit `unit` of the evaluation `evaluation`: the
/// question prepared under the lock of the database, sent without it, the
/// answer written under it again.
///
/// Input:
/// - `database`: the lock of the open database of the session.
/// - `evaluation`: an evaluation in progress; `unit`: one of its work units.
///
/// Output: the label of the work unit, its verdict (`None` when it was not
/// assessed) and the notes: what was not taken of the answer, or why it was
/// not assessed (`("ADV_ARC.1-1", Some(Pass), [])`). An error when the
/// session or the database fails.
///
/// Called by: `evaluation_plan_ai.rs`: `assess_evaluation`.
/// Calls: [`prepared_question`], `objects/ai/integration/integration_ai/sending.rs`:
/// `send_whole`, [`written`].
pub async fn assess_work_unit(
    database: &Mutex<Option<UserDatabase>>,
    evaluation: i64,
    unit: i64,
) -> Result<(String, Option<Verdict>, Vec<String>), AiError> {
    let (label, prepared) = prepared_question(database, evaluation, unit)?;
    let prepared = match prepared {
        Ok(prepared) => prepared,
        Err(reason) => return Ok((label, None, vec![reason])),
    };
    let sent = send_whole(&prepared).await;
    let (verdict, notes) = written(database, (evaluation, unit), prepared, sent)?;

    Ok((label, verdict, notes))
}
