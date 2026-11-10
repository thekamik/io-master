//! The assessment of one work unit of an evaluation by the AI (user,
//! 2026-10-30), as a step of the assessment of the evaluation
//! (`evaluation_plan_ai.rs`: `assess_evaluation`).
//!
//! ```text
//! assess_work_unit ──► prepared_question   (the lock of the database held)  the question with the documents
//!                                           whole, or their passages when they do not fit
//!                  ──► send_until_stopped  (released)                        the answer of the AI, ended by "Stop"
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
use crate::objects::ai::integration::integration_ai::sending::Sent;
use crate::objects::ai::journal::journal_ai::{record, why_not_taken};
use crate::objects::ai_error::AiError;
use crate::objects::evaluation::work_unit::work_unit_ai::assessment_answer::{apply_assessment, read_assessment};
use crate::objects::evaluation::work_unit::work_unit_ai::assessment_question::assessment_question;
use crate::objects::evaluation::work_unit::work_unit_model::Verdict;
use crate::objects::evaluation::work_unit::work_unit_store::read_work_unit;
use crate::objects::session::progress::progress_ai::lock::{lock_database, open_database};
use crate::objects::session::progress::progress_ai::stop::send_until_stopped;
use crate::objects::session::progress::progress_model::Progress;

/// A question prepared, with the notes on how it was made (its documents
/// given as passages), which are kept with the results.
type PreparedWithNotes = (Prepared, Vec<String>);

/// The question of the work unit `unit`, prepared: with its documents whole,
/// or with their passages when they do not fit into the context of the model.
///
/// Input: `evaluation`, the id of an evaluation in progress; `unit`, the id
/// of one of its work units.
/// Output: the question prepared and the notes on how it was made: a note
/// when the documents went as passages, naming the limit of the model
/// (`(prepared, ["The documents did not fit ..."])`); the refusals of
/// `objects/ai/integration/integration_ai/question.rs`: `prepare` (no key, no room even
/// with the passages) and of the question made.
///
/// Called by: [`prepared_question`].
/// Calls: `objects/evaluation/work_unit/work_unit_ai/assessment_question.rs`: `assessment_question`;
/// `objects/ai/integration/integration_ai/question.rs`: `prepare`.
pub(in crate::objects::evaluation) fn prepared_assessment(
    db: &UserDatabase,
    evaluation: i64,
    unit: i64,
) -> Result<PreparedWithNotes, AiError> {
    match prepare(db, &assessment_question(db, evaluation, unit, true)?) {
        Ok(prepared) => Ok((prepared, Vec::new())),
        Err(AiError::TooLong { estimated, room }) => {
            // The evaluator is told that the model saw only parts of the
            // documents, and what would let it see them whole (user, 2026-11-02).
            let note = format!(
                "The documents did not fit the context of the model (about {estimated} tokens, room for {room}): only \
                 their passages that fit the work unit were given. Split the documents into smaller parts, or choose \
                 a model with a larger context in the AI settings."
            );

            Ok((prepare(db, &assessment_question(db, evaluation, unit, false)?)?, vec![note]))
        }
        Err(error) => Err(error),
    }
}

/// Reads the answer `answer` and writes it into the work unit `unit`.
///
/// Input: `evaluation`, the id of an evaluation in progress; `unit`, the id
/// of one of its work units; `answer`, the text of the answer of the AI;
/// `context`, the notes on how the question was made.
/// Output: the notes written beside the model's own (`["a quote not found in
/// the ST"]`); `AiError::Refused` for an answer that cannot be read, an error
/// of the database.
///
/// Called by: [`written`].
/// Calls: `objects/evaluation/work_unit/work_unit_ai/assessment_answer.rs`: `read_assessment`, `apply_assessment`.
pub(in crate::objects::evaluation) fn assessed(
    db: &UserDatabase,
    evaluation: i64,
    unit: i64,
    answer: &str,
    context: &[String],
) -> Result<Vec<String>, AiError> {
    apply_assessment(db, evaluation, unit, &read_assessment(answer)?, context)
}

/// The label of the work unit `unit` and its question, prepared, under the
/// lock of the database.
///
/// Input: `database`, the lock of the open database of the session;
/// `evaluation`, `unit`, as [`prepared_assessment`].
/// Output: the label and the question with the notes on how it was made
/// (`("ADV_ARC.1-1", Ok((prepared, [])))`), or the reason instead of the
/// question when it is refused (no room in the context, no key, no evidence);
/// an error when the session or the database fails.
///
/// Called by: [`assess_work_unit`].
/// Calls: `objects/session/progress/progress_ai/lock.rs`: `lock_database`, `open_database`;
/// `objects/evaluation/work_unit/work_unit_store.rs`: `read_work_unit`; [`prepared_assessment`].
fn prepared_question(
    database: &Mutex<Option<UserDatabase>>,
    evaluation: i64,
    unit: i64,
) -> Result<(String, Result<PreparedWithNotes, String>), AiError> {
    let opened = lock_database(database)?;
    let db = open_database(opened.as_ref())?;
    let label = read_work_unit(db, unit)?.label;

    Ok((label, prepared_assessment(db, evaluation, unit).map_err(|error| error.to_string())))
}

/// Writes the answer into the journal of the agent, then into the work unit,
/// under the lock of the database.
///
/// Input: `database`, the lock of the open database of the session; `step`,
/// the ids of the evaluation and of the work unit; `prepared`, the question
/// as it was sent, with the notes on how it was made; `sent`, its answer.
/// Output: the verdict and the notes (`(Some(Pass), [])`), or no verdict and
/// why (`(None, ["no connection"])`): an answer that failed, or did not end by
/// itself (cut, refused, stopped), is not taken; a question the service
/// seems to have shortened is taken with the warning among its notes; an
/// error when the session or the database fails.
///
/// Called by: [`assess_work_unit`].
/// Calls: `objects/session/progress/progress_ai/lock.rs`: `lock_database`, `open_database`;
/// `objects/ai/journal/journal_ai.rs`: `record`, `why_not_taken`; [`assessed`];
/// `objects/evaluation/work_unit/work_unit_store.rs`: `read_work_unit`.
fn written(
    database: &Mutex<Option<UserDatabase>>,
    step: (i64, i64),
    prepared: PreparedWithNotes,
    sent: Sent,
) -> Result<(Option<Verdict>, Vec<String>), AiError> {
    let (evaluation, unit) = step;
    let (prepared, mut context) = prepared;
    let opened = lock_database(database)?;
    let db = open_database(opened.as_ref())?;
    let answer = record(db, prepared, sent)?;

    if let Some(error) = answer.error {
        return Ok((None, vec![error.to_string()]));
    }

    if let Some(why) = why_not_taken(answer.ending) {
        return Ok((None, vec![format!("The answer was not taken: {why}.")]));
    }

    // The evaluator is told when the model may have judged documents it did
    // not read whole: the warning is kept with the results.
    if let Some(warning) = answer.shortened {
        context.push(warning);
    }

    Ok(match assessed(db, evaluation, unit, &answer.text, &context) {
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
/// - `progress`: the lock of the progress of the session: a "Stop" of the
///   user ends the answer awaited.
/// - `evaluation`: an evaluation in progress; `unit`: one of its work units.
///
/// Output: the label of the work unit, its verdict (`None` when it was not
/// assessed) and the notes: what was not taken of the answer, or why it was
/// not assessed (`("ADV_ARC.1-1", Some(Pass), [])`). An error when the
/// session or the database fails.
///
/// Called by: `objects/evaluation/evaluation_plan/evaluation_plan_ai.rs`: `assess_evaluation`.
/// Calls: [`prepared_question`]; `objects/session/progress/progress_ai/stop.rs`: `send_until_stopped`;
/// [`written`].
pub async fn assess_work_unit(
    database: &Mutex<Option<UserDatabase>>,
    progress: &Mutex<Progress>,
    evaluation: i64,
    unit: i64,
) -> Result<(String, Option<Verdict>, Vec<String>), AiError> {
    let (label, prepared) = prepared_question(database, evaluation, unit)?;
    let prepared = match prepared {
        Ok(prepared) => prepared,
        Err(reason) => return Ok((label, None, vec![reason])),
    };
    let sent = send_until_stopped(&prepared.0, progress).await;
    let (verdict, notes) = written(database, (evaluation, unit), prepared, sent)?;

    Ok((label, verdict, notes))
}
