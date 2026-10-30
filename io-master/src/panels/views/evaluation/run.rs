//! The assessment of an evaluation by the AI, as the page calls it (user,
//! 2026-10-30): every work unit in the order of the work, one after the
//! other, without the evaluator. The page sees each step as it ends and may
//! stop the assessment.
//!
//! ```text
//! EvaluationUnits.svelte ──evaluation_assess_preview(id, again)─► preview ──► AssessPreviewView
//!                        ──evaluation_assess(id, again, on_step)─► preview (refused while an input is missing),
//!                            then for each work unit:
//!                            (the lock of the session held)  prepared_assessment
//!                            (released)                      answer::send_drained
//!                            (held)                          ask::record (the journal of the agent), assessed
//!                            ──on_step──► AssessStepView
//!                        ──evaluation_assess_stop()──────────► the assessment ends after its present work unit
//! ```
//!
//! Only one long work of the AI runs at a time (`Session::begin_work`); a
//! work unit the AI could not assess is told and passed over; a failure of
//! the session or of the database ends the assessment.

use serde::Serialize;
use tauri::State;
use tauri::ipc::Channel;

use crate::ai::ask::{Prepared, Sent, record};
use crate::db::evaluation::work_unit::read_work_unit;
use crate::panels::logic::evaluation::question::verdict_name;
use crate::panels::logic::evaluation::run::{Preview, assessed, prepared_assessment, preview};
use crate::panels::views::answer::send_drained;
use crate::panels::views::error::ViewError;
use crate::panels::views::opened::database;
use crate::session::state::Session;

/// A name and how many questions or work units it counts.
#[derive(Serialize)]
pub struct CountView {
    name: String,
    count: usize,
}

/// What an assessment would ask, counted when the evaluator asks it.
#[derive(Serialize)]
pub struct AssessPreviewView {
    questions: usize,
    agents: Vec<CountView>,
    /// The documents and proofs needed and not given: the assessment does
    /// not start while there is one.
    missing: Vec<CountView>,
}

/// One step of an assessment, as the page shows it.
#[derive(Serialize)]
pub struct AssessStepView {
    /// The place of the work unit in the assessment, from 1, and how many
    /// there are.
    number: usize,
    total: usize,
    label: String,
    /// "pass", "fail", "inconclusive"; `None` when it was not assessed, and
    /// `notes` tell why.
    verdict: Option<&'static str>,
    /// What was not taken from the answer, or why it was not assessed.
    notes: Vec<String>,
}

/// How an assessment ended.
#[derive(Serialize)]
pub struct AssessSummaryView {
    assessed: usize,
    failed: usize,
    /// The user stopped it before its end.
    stopped: bool,
}

/// The counts of `counted` as the page shows them.
///
/// Called by: [`preview_view`].
fn count_views(counted: Vec<(String, usize)>) -> Vec<CountView> {
    let mut views = Vec::with_capacity(counted.len());

    for (name, count) in counted {
        views.push(CountView { name, count });
    }

    views
}

/// The preview `found` as the page shows it.
///
/// Called by: [`evaluation_assess_preview`].
/// Calls: [`count_views`].
fn preview_view(found: Preview) -> AssessPreviewView {
    AssessPreviewView {
        questions: found.units.len(),
        agents: count_views(found.agents),
        missing: count_views(found.missing),
    }
}

/// What the assessment of the evaluation `id` would ask, counted now; with
/// `again` the work units of the AI assessed already too.
///
/// Called by: `EvaluationUnits.svelte`, before an assessment starts.
/// Calls: `preview`, [`preview_view`].
#[tauri::command]
pub fn evaluation_assess_preview(
    session: State<'_, Session>,
    id: i64,
    again: bool,
) -> Result<AssessPreviewView, ViewError> {
    let opened = session.lock()?;

    Ok(preview_view(preview(database(&opened)?, id, again)?))
}

/// The label of the work unit `unit` and its question, prepared; the reason
/// when the question is refused.
///
/// Called by: [`assess_step`].
/// Calls: `read_work_unit`, `prepared_assessment`.
fn prepared_question(
    session: &Session,
    evaluation: i64,
    unit: i64,
) -> Result<(String, Result<Prepared, String>), ViewError> {
    let opened = session.lock()?;
    let db = database(&opened)?;
    let label = read_work_unit(db, unit)?.label;

    Ok((label, prepared_assessment(db, evaluation, unit).map_err(|error| error.to_string())))
}

/// Writes the answer into the journal of the agent, then into the work unit
/// `unit`; gives its verdict and the notes, or no verdict and why.
///
/// Called by: [`assess_step`].
/// Calls: `ask::record`, `assessed`, `read_work_unit`.
fn written(
    session: &Session,
    step: (i64, i64),
    prepared: Prepared,
    sent: Sent,
) -> Result<(Option<&'static str>, Vec<String>), ViewError> {
    let (evaluation, unit) = step;
    let opened = session.lock()?;
    let db = database(&opened)?;
    let answer = record(db, prepared, sent)?;

    if let Some(error) = answer.error {
        return Ok((None, vec![error.to_string()]));
    }

    Ok(match assessed(db, evaluation, unit, &answer.text) {
        Ok(notes) => (read_work_unit(db, unit)?.verdict.map(verdict_name), notes),
        Err(error) => (None, vec![error.to_string()]),
    })
}

/// Assesses the work unit `unit`: the question prepared under the lock of
/// the session, sent without it, the answer written under it again.
///
/// Called by: [`assess`].
/// Calls: [`prepared_question`], `answer::send_drained`, [`written`].
async fn assess_step(
    session: &Session,
    evaluation: i64,
    unit: i64,
) -> Result<(String, Option<&'static str>, Vec<String>), ViewError> {
    let (label, prepared) = prepared_question(session, evaluation, unit)?;
    let prepared = match prepared {
        Ok(prepared) => prepared,
        Err(reason) => return Ok((label, None, vec![reason])),
    };
    let sent = send_drained(&prepared).await;
    let (verdict, notes) = written(session, (evaluation, unit), prepared, sent)?;

    Ok((label, verdict, notes))
}

/// The work units to assess, refused while a document or a proof they need
/// is not given.
///
/// Called by: [`assess`].
/// Calls: `preview`.
fn units_ready(session: &Session, evaluation: i64, again: bool) -> Result<Vec<i64>, ViewError> {
    let opened = session.lock()?;
    let found = preview(database(&opened)?, evaluation, again)?;
    let mut missing = Vec::with_capacity(found.missing.len());

    for (name, count) in &found.missing {
        missing.push(format!("{name} ({count} work units)"));
    }

    ensure!(
        missing.is_empty(),
        ViewError::Invalid(format!("give first what the work units need: {}", missing.join(", ")))
    );

    Ok(found.units)
}

/// Assesses every work unit to assess of the evaluation `id` in the order of
/// the work, telling the page each step through `on_step`; with `again` the
/// work units of the AI assessed already too. Refused while another work of
/// the AI runs, or an input is missing.
///
/// Called by: `EvaluationUnits.svelte`, "Assess with AI", "Assess again".
/// Calls: `Session::begin_work`, [`units_ready`], [`assess_step`].
#[tauri::command]
pub async fn evaluation_assess(
    session: State<'_, Session>,
    id: i64,
    again: bool,
    on_step: Channel<AssessStepView>,
) -> Result<AssessSummaryView, ViewError> {
    let Some(_work) = session.begin_work() else {
        return Err(ViewError::Invalid("another work of the AI runs: wait for it or stop it".to_owned()));
    };
    let units = units_ready(&session, id, again)?;
    let mut summary = AssessSummaryView { assessed: 0, failed: 0, stopped: false };

    for (index, unit) in units.iter().enumerate() {
        if session.stop_requested() {
            summary.stopped = true;
            break;
        }

        let (label, verdict, notes) = assess_step(&session, id, *unit).await?;

        if verdict.is_some() {
            summary.assessed += 1;
        } else {
            summary.failed += 1;
        }

        on_step
            .send(AssessStepView { number: index + 1, total: units.len(), label, verdict, notes })
            .map_err(|error| ViewError::Invalid(format!("the page does not listen: {error}")))?;
    }

    Ok(summary)
}

/// Asks the assessment that runs to stop after its present work unit.
///
/// Called by: `EvaluationUnits.svelte`, "Stop".
/// Calls: `Session::request_stop`.
#[tauri::command]
pub fn evaluation_assess_stop(session: State<'_, Session>) {
    session.request_stop();
}
