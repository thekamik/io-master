//! "Fill with AI", as the page calls it (user, 2026-10-29): the agent of
//! the planner fills every work unit of the plan, one after the other; the
//! page sees each step as it ends and may stop the filling.
//!
//! ```text
//! PlanEditor.svelte ──plan_fill(plan, on_step)──► fill_units
//!                     for each work unit:          (the lock of the session held)  fill_question, ask::prepare
//!                                                  (released)                      answer::send_drained
//!                                                  (held)                          ask::record (the journal of the agent),
//!                                                                                  read_proposal, apply_proposal
//!                                                  ──on_step──► FillStepView (the work unit, filled or not, the notes)
//!                   ──plan_fill_stop()──────────► the session asked to stop: the filling ends after its present step
//! ```
//!
//! Only one filling runs at a time (`Session::begin_work`). A work unit the
//! AI could not fill is told and passed over; a failure of the session or
//! of the database ends the filling.

use serde::Serialize;
use tauri::State;
use tauri::ipc::Channel;

use crate::ai::ask::{Prepared, Sent, prepare, record};
use crate::db::evaluation::work_unit::read_work_unit;
use crate::panels::logic::evaluation_plan::fill::{apply_proposal, fill_question, fill_units, read_proposal};
use crate::panels::views::answer::send_drained;
use crate::panels::views::error::ViewError;
use crate::panels::views::opened::database;
use crate::session::state::Session;

/// One step of a filling, as the page shows it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FillStepView {
    /// The place of the work unit in the plan, from 1, and how many there
    /// are.
    number: usize,
    total: usize,
    label: String,
    /// The work unit was filled; otherwise `notes` tell why it was not.
    filled: bool,
    /// What was left out of the answer, or why the work unit was not filled.
    notes: Vec<String>,
}

/// How a filling ended.
#[derive(Serialize)]
pub struct FillSummaryView {
    filled: usize,
    failed: usize,
    /// The user stopped it before its end.
    stopped: bool,
}

/// What became of one work unit.
enum Outcome {
    /// Filled; what was left out of the answer.
    Filled(Vec<String>),
    /// Not filled; why.
    Failed(String),
}

/// The label of the work unit `unit` and its question, prepared; the
/// reason when the question is refused (no room in the context, no key).
///
/// Called by: [`fill_step`].
/// Calls: `read_work_unit`, `fill_question`, `ask::prepare`.
fn prepared_question(session: &Session, plan: i64, unit: i64) -> Result<(String, Result<Prepared, String>), ViewError> {
    let opened = session.lock()?;
    let db = database(&opened)?;
    let label = read_work_unit(db, unit)?.label;
    let prepared = match fill_question(db, plan, unit) {
        Ok(question) => prepare(db, &question).map_err(|error| error.to_string()),
        Err(error) => Err(error.to_string()),
    };

    Ok((label, prepared))
}

/// Writes the answer into the journal of the agent of the planner, then
/// its proposal into the work unit `unit`.
///
/// Called by: [`fill_step`].
/// Calls: `ask::record`, `read_proposal`, `apply_proposal`.
fn written(session: &Session, plan: i64, unit: i64, prepared: Prepared, sent: Sent) -> Result<Outcome, ViewError> {
    let opened = session.lock()?;
    let db = database(&opened)?;
    let answer = record(db, prepared, sent)?;

    if let Some(error) = answer.error {
        return Ok(Outcome::Failed(error.to_string()));
    }

    let outcome = match read_proposal(&answer.text) {
        Ok(proposal) => match apply_proposal(db, plan, unit, &proposal) {
            Ok(notes) => Outcome::Filled(notes),
            Err(error) => Outcome::Failed(error.to_string()),
        },
        Err(error) => Outcome::Failed(error.to_string()),
    };

    Ok(outcome)
}

/// Fills the work unit `unit`: the question prepared under the lock of the
/// session, sent without it, the answer written under it again.
///
/// Called by: [`plan_fill`].
/// Calls: [`prepared_question`], `answer::send_drained`, [`written`].
async fn fill_step(session: &Session, plan: i64, unit: i64) -> Result<(String, Outcome), ViewError> {
    let (label, prepared) = prepared_question(session, plan, unit)?;
    let prepared = match prepared {
        Ok(prepared) => prepared,
        Err(reason) => return Ok((label, Outcome::Failed(reason))),
    };
    let sent = send_drained(&prepared).await;

    Ok((label, written(session, plan, unit, prepared, sent)?))
}

/// Fills every work unit of the plan `plan` in its order with the agent of
/// its planner, over what was there; tells the page each step through
/// `on_step`. Refused while another filling runs, or when the plan has no
/// agent of the planner.
///
/// Called by: `PlanEditor.svelte`, "Fill with AI".
/// Calls: `Session::begin_work`, `fill_units`, [`fill_step`].
#[tauri::command]
pub async fn plan_fill(
    session: State<'_, Session>,
    plan: i64,
    on_step: Channel<FillStepView>,
) -> Result<FillSummaryView, ViewError> {
    let Some(_work) = session.begin_work() else {
        return Err(ViewError::Invalid("a filling runs already".to_owned()));
    };
    let units = {
        let opened = session.lock()?;

        fill_units(database(&opened)?, plan)?
    };
    let mut summary = FillSummaryView { filled: 0, failed: 0, stopped: false };

    for (index, unit) in units.iter().enumerate() {
        if session.stop_requested() {
            summary.stopped = true;
            break;
        }

        let (label, outcome) = fill_step(&session, plan, *unit).await?;
        let (filled, notes) = match outcome {
            Outcome::Filled(notes) => (true, notes),
            Outcome::Failed(reason) => (false, vec![reason]),
        };

        if filled {
            summary.filled += 1;
        } else {
            summary.failed += 1;
        }

        on_step
            .send(FillStepView { number: index + 1, total: units.len(), label, filled, notes })
            .map_err(|error| ViewError::Invalid(format!("the page does not listen: {error}")))?;
    }

    Ok(summary)
}

/// Asks the filling that runs to stop after its present work unit.
///
/// Called by: `PlanEditor.svelte`, "Stop".
/// Calls: `Session::request_stop`.
#[tauri::command]
pub fn plan_fill_stop(session: State<'_, Session>) {
    session.request_stop();
}
