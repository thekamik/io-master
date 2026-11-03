//! The reports of the panel Reports, as the page calls them: the
//! evaluations to report on, the report of one by a template, shown and
//! written into a file the user chooses.
//!
//! ```text
//! Reports.svelte ──report_list()──────────────────────► the evaluations and reports ──► [ReportSummaryView]
//!                ──report_preview(evaluation, template)► filled_report ──► the text
//!                ──report_export(evaluation, template)─► the dialog of the system ──► write_report ──► whether written
//! ```

use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use crate::objects::evaluation::evaluation_plan::evaluation_plan_model::{EvaluationPlan, PlanState};
use crate::objects::evaluation::evaluation_plan::evaluation_plan_store::{read_evaluation_plan, read_evaluation_plans};
use crate::objects::evaluation::report_template::report_template_rules::filling::{filled_report, write_report};
use crate::objects::evaluation::report_template::report_template_store::read_report_template;
use crate::panels::session::Session;
use crate::panels::views::error::ViewError;
use crate::panels::views::opened::database;

/// An evaluation to report on, as the list shows it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportSummaryView {
    id: i64,
    /// The name of the plan it was copied from.
    name: String,
    product: String,
    client: String,
    evaluator: String,
    /// Seconds since 1970 (UTC).
    started: Option<i64>,
    /// "evaluation" or "report".
    state: &'static str,
    work_units: usize,
}

/// The summary of the evaluation `plan` as the list of the reports shows it.
///
/// Input: `plan`, an evaluation or a report.
/// Output: its summary (`ReportSummaryView { id: 12, product: "Router R7", .. }`).
///
/// Called by: [`report_list`].
fn summary_of(plan: EvaluationPlan) -> ReportSummaryView {
    let state = if plan.state == PlanState::Report { "report" } else { "evaluation" };

    ReportSummaryView {
        id: plan.id,
        name: plan.name,
        product: plan.product,
        client: plan.client,
        evaluator: plan.evaluator,
        started: plan.started,
        state,
        work_units: plan.work_units.len(),
    }
}

/// Every evaluation and report, the newest first: what a report can be
/// written of.
///
/// Input: `session`, the session of the window, someone logged in.
/// Output: the summaries (`[{"id": 12, "product": "Router R7", ...}]`);
/// `ViewError::NotLoggedIn` when nobody is logged in, an error of the database.
///
/// Called by: `ui/src/lib/api/reports.ts`: `reportList`, from `ui/src/views/reports/Reports.svelte`, when
/// the panel opens.
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/evaluation/evaluation_plan/evaluation_plan_store.rs`: `read_evaluation_plans`; [`summary_of`].
#[tauri::command]
pub fn report_list(session: State<'_, Session>) -> Result<Vec<ReportSummaryView>, ViewError> {
    let opened = session.lock()?;
    let mut evaluations = Vec::new();

    for plan in read_evaluation_plans(database(opened.as_ref())?)? {
        if plan.state != PlanState::Plan {
            evaluations.push(summary_of(plan));
        }
    }

    Ok(evaluations)
}

/// The report of the evaluation `evaluation` by the template `template`,
/// to be shown before it is written.
///
/// Input: `session`, the session of the window, someone logged in;
/// `evaluation`, the id of an evaluation; `template`, the id of a template.
/// Output: the report ("# Evaluation report: Router R7\n..."); `ViewError::NotLoggedIn`
/// when nobody is logged in, the refusals of the report.
///
/// Called by: `ui/src/lib/api/reports.ts`: `reportPreview`, from `ui/src/views/reports/Reports.svelte`,
/// when an evaluation and a template are chosen.
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/evaluation/report_template/report_template_rules/filling.rs`: `filled_report`.
#[tauri::command]
pub fn report_preview(session: State<'_, Session>, evaluation: i64, template: i64) -> Result<String, ViewError> {
    let opened = session.lock()?;

    Ok(filled_report(database(opened.as_ref())?, evaluation, template)?)
}

/// The name a report is offered under in the dialog: the product, the name
/// of the template and its extension ("Router R7 - ETR.md").
///
/// Input: `session`, the session of the window, someone logged in;
/// `evaluation`, `template`, as [`report_export`].
/// Output: the name of the file; an error of the session or the database.
///
/// Called by: [`report_export`].
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/evaluation/evaluation_plan/evaluation_plan_store.rs`: `read_evaluation_plan`;
/// `objects/evaluation/report_template/report_template_store.rs`: `read_report_template`.
fn file_name(session: &Session, evaluation: i64, template: i64) -> Result<String, ViewError> {
    let opened = session.lock()?;
    let db = database(opened.as_ref())?;
    let plan = read_evaluation_plan(db, evaluation)?;
    let template = read_report_template(db, template)?;
    // A slash in a product would be a folder in the name of a file.
    let product = plan.product.replace('/', "-");

    Ok(format!("{product} - {}.{}", template.name.replace('/', "-"), template.extension))
}

/// Writes the report of the evaluation `evaluation` by the template
/// `template` into a file the user chooses in the dialog of the system;
/// gives whether it was written. Run apart from the window (`async`): the
/// dialog waits for the user, and the session is not held meanwhile.
///
/// Input: `app`, the handle of the window; `session`, the session of the
/// window, someone logged in; `evaluation`, the id of an evaluation;
/// `template`, the id of a template.
/// Output: whether a file was chosen and written (`true`); `ViewError::NotLoggedIn`
/// when nobody is logged in, the refusals of the report or of the file.
///
/// Called by: `ui/src/lib/api/reports.ts`: `reportExport`, from `ui/src/views/reports/Reports.svelte`,
/// "Export".
/// Calls: [`file_name`]; the dialog of the system; `panels/session.rs`: `Session::lock`;
/// `panels/views/opened.rs`: `database`;
/// `objects/evaluation/report_template/report_template_rules/filling.rs`: `write_report`.
#[tauri::command(async)]
pub fn report_export(
    app: AppHandle,
    session: State<'_, Session>,
    evaluation: i64,
    template: i64,
) -> Result<bool, ViewError> {
    let name = file_name(&session, evaluation, template)?;
    let Some(file) = app.dialog().file().set_title("Export the report").set_file_name(name).blocking_save_file() else {
        return Ok(false);
    };
    let path = file.into_path().map_err(|error| ViewError::Invalid(format!("the file chosen: {error}")))?;
    let opened = session.lock()?;

    write_report(database(opened.as_ref())?, evaluation, template, &path)?;

    Ok(true)
}
