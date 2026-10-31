//! The work units of an evaluation, as the page shows them: the list in the
//! order of the work, one work unit with its planning (read only), its
//! results and the last question and answer of the AI about it, and the
//! results as the evaluator corrects them.
//!
//! ```text
//! EvaluationUnits.svelte ──evaluation_unit_list(id)──────────► units_in_order ──► [UnitRowView]
//! EvaluationUnit.svelte  ──evaluation_unit_get(id, unit)─────► the work unit, its names, the journal ──► UnitView
//!                        ──evaluation_unit_save(id, unit, ...)► save_results (refused while the AI works)
//! ```

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::db::models::ai::agent::read_agent;
use crate::db::models::ai::journal::read_latest_journal_entry;
use crate::db::models::evaluation::evaluation_plan::read_evaluation_plan;
use crate::db::models::evaluation::lab_proof::read_lab_proof;
use crate::db::models::evaluation::oscal_standard_document::read_oscal_standard_document;
use crate::db::models::evaluation::product_document::read_product_document;
use crate::db::models::evaluation::work_unit::read_work_unit;
use crate::db::storage::user_database::UserDatabase;
use crate::model::ai::journal::Ending;
use crate::model::evaluation::evaluation_plan::EvaluationPlan;
use crate::model::evaluation::work_unit::{Author, Quote, Verdict, WorkUnit};
use crate::panels::logic::evaluation::question::{assessing_agent, verdict_name};
use crate::panels::logic::evaluation::results::{Results, save_results};
use crate::panels::logic::evaluation::run::units_in_order;
use crate::panels::views::error::ViewError;
use crate::panels::views::opened::database;
use crate::session::state::Session;

/// The longest text of the results from the page, in bytes (the logic
/// checks the characters).
const MAX_RESULTS_BYTES: usize = 2 * 1024 * 1024;

/// A work unit in the list of an evaluation.
#[derive(Serialize)]
pub struct UnitRowView {
    id: i64,
    label: String,
    functional: bool,
    /// The name of the agent that assesses it.
    agent: String,
    /// Whether that agent is the one of the evaluation.
    general: bool,
    verdict: Option<&'static str>,
    /// "ai" or "user".
    author: Option<&'static str>,
}

/// A name with its id, and what it tells (a verdict, a version).
#[derive(Serialize)]
pub struct NamedView {
    id: i64,
    name: String,
    detail: Option<String>,
}

/// A quote, as the page shows and sends it.
#[derive(Serialize, Deserialize)]
pub struct QuoteView {
    document: i64,
    text: String,
}

/// What a work unit needs, by names.
struct Needs {
    /// Its dependencies with their verdicts.
    dependencies: Vec<NamedView>,
    /// Its documents with their versions.
    documents: Vec<NamedView>,
    /// Its proofs; `None` for a work unit of the documentation.
    proofs: Option<Vec<String>>,
}

/// The last question of the AI about a work unit and its answer.
#[derive(Serialize)]
pub struct JournalView {
    /// Seconds since 1970 (UTC).
    time: i64,
    model: String,
    request: String,
    answer: String,
    /// "complete", "cut", "stopped", "refused", "failed".
    ending: &'static str,
}

/// One work unit of an evaluation whole.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitView {
    id: i64,
    label: String,
    standard: String,
    requirement: String,
    guidance: String,
    dependencies: Vec<NamedView>,
    documents: Vec<NamedView>,
    /// `None` for a work unit of the documentation.
    proofs: Option<Vec<String>>,
    agent: String,
    findings: String,
    analysis: String,
    vulnerability_notes: String,
    notes: String,
    quotes: Vec<QuoteView>,
    verdict: Option<&'static str>,
    author: Option<&'static str>,
    /// The verdicts the plan knows.
    verdicts: Vec<&'static str>,
    journal: Option<JournalView>,
}

/// The name of an ending on the page.
///
/// Called by: [`unit_view`].
fn ending_name(ending: Ending) -> &'static str {
    match ending {
        Ending::Complete => "complete",
        Ending::Cut => "cut",
        Ending::Stopped => "stopped",
        Ending::Refused => "refused",
        Ending::Failed => "failed",
    }
}

/// The name of an author on the page.
///
/// Called by: [`row_of`], [`unit_view`].
fn author_name(author: Author) -> &'static str {
    match author {
        Author::Ai => "ai",
        Author::User => "user",
    }
}

/// The row of the work unit `unit` of the evaluation `evaluation`.
///
/// Called by: [`evaluation_unit_list`].
/// Calls: `assessing_agent`, `read_agent`, [`author_name`].
fn row_of(db: &UserDatabase, evaluation: &EvaluationPlan, unit: WorkUnit) -> Result<UnitRowView, ViewError> {
    let agent = read_agent(db, assessing_agent(evaluation, &unit)?)?.name;

    Ok(UnitRowView {
        id: unit.id,
        label: unit.label,
        functional: unit.input_functional.is_some(),
        agent,
        general: unit.ai_agent.is_none(),
        verdict: unit.verdict.map(verdict_name),
        author: unit.author.map(author_name),
    })
}

/// The work units of the evaluation `id` in the order of the work.
///
/// Called by: `EvaluationUnits.svelte`, when the tab opens and after each
/// step of an assessment.
/// Calls: `read_evaluation_plan`, `units_in_order`, [`row_of`].
#[tauri::command]
pub fn evaluation_unit_list(session: State<'_, Session>, id: i64) -> Result<Vec<UnitRowView>, ViewError> {
    let opened = session.lock()?;
    let db = database(&opened)?;
    let evaluation = read_evaluation_plan(db, id)?;
    let units = units_in_order(db, id)?;
    let mut rows = Vec::with_capacity(units.len());

    for unit in units {
        rows.push(row_of(db, &evaluation, unit)?);
    }

    Ok(rows)
}

/// The names of what the work unit `unit` needs: its dependencies with their
/// verdicts, its documents with their versions, its proofs.
///
/// Called by: [`unit_view`].
/// Calls: `read_work_unit`, `read_product_document`, `read_lab_proof`.
fn needs_of(db: &UserDatabase, unit: &WorkUnit) -> Result<Needs, ViewError> {
    let mut dependencies = Vec::with_capacity(unit.dependencies.len());
    let mut documents = Vec::with_capacity(unit.input_product_documents.len());

    for id in &unit.dependencies {
        let dependency = read_work_unit(db, *id)?;
        let detail = dependency.verdict.map(|verdict| verdict_name(verdict).to_owned());

        dependencies.push(NamedView { id: *id, name: dependency.label, detail });
    }

    for id in &unit.input_product_documents {
        let document = read_product_document(db, *id)?;

        documents.push(NamedView { id: *id, name: document.name, detail: document.version });
    }

    let proofs = match &unit.input_functional {
        Some(ids) => {
            let mut names = Vec::with_capacity(ids.len());

            for id in ids {
                names.push(read_lab_proof(db, *id)?.name);
            }

            Some(names)
        }
        None => None,
    };

    Ok(Needs { dependencies, documents, proofs })
}

/// The view of the work unit `unit` of the evaluation `evaluation`.
///
/// Called by: [`evaluation_unit_get`].
/// Calls: [`needs_of`], `assessing_agent`, `read_agent`,
/// `read_oscal_standard_document`, `read_latest_journal_entry`,
/// [`ending_name`], [`author_name`].
fn unit_view(db: &UserDatabase, evaluation: &EvaluationPlan, unit: WorkUnit) -> Result<UnitView, ViewError> {
    let Needs { dependencies, documents, proofs } = needs_of(db, &unit)?;
    let agent = read_agent(db, assessing_agent(evaluation, &unit)?)?.name;
    let journal = read_latest_journal_entry(db, unit.id)?.map(|entry| JournalView {
        time: entry.time,
        model: entry.model,
        request: entry.request,
        answer: entry.answer,
        ending: ending_name(entry.ending),
    });
    let mut quotes = Vec::with_capacity(unit.quotes.len());
    let mut verdicts = Vec::with_capacity(evaluation.possible_verdicts.len());

    for quote in unit.quotes {
        quotes.push(QuoteView { document: quote.product_document, text: quote.text });
    }

    for verdict in &evaluation.possible_verdicts {
        verdicts.push(verdict_name(*verdict));
    }

    Ok(UnitView {
        id: unit.id,
        standard: read_oscal_standard_document(db, unit.reference)?.title,
        label: unit.label,
        requirement: unit.requirement,
        guidance: unit.guidance,
        dependencies,
        documents,
        proofs,
        agent,
        findings: unit.findings,
        analysis: unit.analysis,
        vulnerability_notes: unit.vulnerability_notes,
        notes: unit.notes,
        quotes,
        verdict: unit.verdict.map(verdict_name),
        author: unit.author.map(author_name),
        verdicts,
        journal,
    })
}

/// The work unit `unit` of the evaluation `id` whole.
///
/// Called by: `EvaluationUnit.svelte`, when a work unit is opened.
/// Calls: `read_evaluation_plan`, `read_work_unit`, [`unit_view`].
#[tauri::command]
pub fn evaluation_unit_get(session: State<'_, Session>, id: i64, unit: i64) -> Result<UnitView, ViewError> {
    let opened = session.lock()?;
    let db = database(&opened)?;
    let evaluation = read_evaluation_plan(db, id)?;

    ensure!(
        evaluation.work_units.contains(&unit),
        ViewError::Invalid("the work unit is not one of the evaluation".to_owned())
    );

    unit_view(db, &evaluation, read_work_unit(db, unit)?)
}

/// The verdict of its name on the page; `None` for an empty name; a name the
/// page never sends is refused.
///
/// Called by: [`evaluation_unit_save`].
fn verdict_of(name: &str) -> Result<Option<Verdict>, ViewError> {
    match name {
        "" => Ok(None),
        "pass" => Ok(Some(Verdict::Pass)),
        "fail" => Ok(Some(Verdict::Fail)),
        "inconclusive" => Ok(Some(Verdict::Inconclusive)),
        _ => Err(ViewError::Invalid(format!("no verdict \"{name}\""))),
    }
}

/// The results of a work unit as the page sends them.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultsFields {
    findings: String,
    analysis: String,
    vulnerability_notes: String,
    notes: String,
    quotes: Vec<QuoteView>,
    /// "pass", "fail", "inconclusive", or empty for none.
    verdict: String,
}

/// Saves the results of the work unit `unit` of the evaluation `id` as the
/// evaluator gives them (author User). Refused while a work of the AI runs:
/// the evaluator and the AI never write at once (user, 2026-10-30).
///
/// Called by: `EvaluationUnit.svelte`, "Save".
/// Calls: `Session::begin_work`, [`verdict_of`], `save_results`.
#[tauri::command]
pub fn evaluation_unit_save(
    session: State<'_, Session>,
    id: i64,
    unit: i64,
    fields: ResultsFields,
) -> Result<(), ViewError> {
    let mut size =
        fields.findings.len() + fields.analysis.len() + fields.vulnerability_notes.len() + fields.notes.len();

    for quote in &fields.quotes {
        size += quote.text.len();
    }

    ensure!(size <= MAX_RESULTS_BYTES, ViewError::Invalid("the results are too long".to_owned()));

    let Some(_work) = session.begin_work() else {
        return Err(ViewError::Invalid("the AI is working: save when it has ended".to_owned()));
    };
    let mut quotes = Vec::with_capacity(fields.quotes.len());

    for quote in fields.quotes {
        quotes.push(Quote { product_document: quote.document, text: quote.text });
    }

    let results = Results {
        findings: fields.findings,
        analysis: fields.analysis,
        vulnerability_notes: fields.vulnerability_notes,
        notes: fields.notes,
        quotes,
        verdict: verdict_of(&fields.verdict)?,
    };
    let opened = session.lock()?;

    Ok(save_results(database(&opened)?, id, unit, &results)?)
}
