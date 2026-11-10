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

use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::ai::agent::agent_store::read_agent;
use crate::objects::ai::journal::journal_model::Ending;
use crate::objects::ai::journal::journal_store::read_latest_journal_entry;
use crate::objects::evaluation::evaluation_plan::evaluation_plan_ai::units_in_order;
use crate::objects::evaluation::evaluation_plan::evaluation_plan_model::EvaluationPlan;
use crate::objects::evaluation::evaluation_plan::evaluation_plan_store::read_evaluation_plan;
use crate::objects::evaluation::lab_proof::lab_proof_store::read_lab_proof;
use crate::objects::evaluation::oscal_standard_document::oscal_standard_document_store::read_oscal_standard_document;
use crate::objects::evaluation::product_document::product_document_store::read_document_record;
use crate::objects::evaluation::work_unit::work_unit_ai::assessment_question::{assessing_agent, verdict_name};
use crate::objects::evaluation::work_unit::work_unit_model::{Author, ProofQuote, Quote, Verdict, WorkUnit};
use crate::objects::evaluation::work_unit::work_unit_rules::results::{Results, save_results};
use crate::objects::evaluation::work_unit::work_unit_store::read_work_unit;
use crate::panels::session::Session;
use crate::panels::views::error::ViewError;
use crate::panels::views::opened::database;

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

/// A quote of a proof of the laboratory, as the page shows and sends it.
#[derive(Serialize, Deserialize)]
pub struct ProofQuoteView {
    proof: i64,
    text: String,
}

/// What a work unit needs, by names.
struct Needs {
    /// Its dependencies with their verdicts.
    dependencies: Vec<NamedView>,
    /// Its documents with their versions.
    documents: Vec<NamedView>,
    /// Its proofs; `None` for a work unit of the documentation.
    proofs: Option<Vec<NamedView>>,
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
    proofs: Option<Vec<NamedView>>,
    agent: String,
    findings: String,
    analysis: String,
    vulnerability_notes: String,
    notes: String,
    quotes: Vec<QuoteView>,
    proof_quotes: Vec<ProofQuoteView>,
    verdict: Option<&'static str>,
    author: Option<&'static str>,
    /// The verdicts the plan knows.
    verdicts: Vec<&'static str>,
    journal: Option<JournalView>,
}

/// The name of an ending on the page.
///
/// Input: `ending`, how an answer ended.
/// Output: its name on the page (`"cut"`).
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
/// Input: `author`, who wrote the results.
/// Output: its name on the page (`"ai"`).
///
/// Called by: [`row_of`], [`unit_view`].
fn author_name(author: Author) -> &'static str {
    match author {
        Author::Ai => "ai",
        Author::User => "user",
    }
}

/// The name of the agent `id`, read once for the whole list: most work units
/// are assessed by the agent of their evaluation, and the list is read again
/// after each step of an assessment.
///
/// Input: `names`, the names read so far, at most one for each agent of the
/// user; `id`, the id of an agent.
/// Output: its name ("Evaluator"); an error of the database for an agent
/// missing.
///
/// Called by: [`row_of`].
/// Calls: `objects/ai/agent/agent_store.rs`: `read_agent`.
fn agent_name(db: &UserDatabase, names: &mut Vec<(i64, String)>, id: i64) -> Result<String, ViewError> {
    for (known, name) in names.iter() {
        if *known == id {
            return Ok(name.clone());
        }
    }

    let name = read_agent(db, id)?.name;

    names.push((id, name.clone()));

    Ok(name)
}

/// The row of the work unit `unit` of the evaluation `evaluation`.
///
/// Input: `db`, the open database; `evaluation`, the evaluation; `unit`,
/// one of its work units; `names`, the names of the agents read so far for
/// the list.
/// Output: its row with its agent (`UnitRowView { label: "ADV_ARC.1-1", .. }`);
/// an error for a work unit without an agent or a database failed.
///
/// Called by: [`evaluation_unit_list`].
/// Calls: `objects/evaluation/work_unit/work_unit_ai/assessment_question.rs`: `assessing_agent`;
/// [`agent_name`], [`author_name`].
fn row_of(
    db: &UserDatabase,
    evaluation: &EvaluationPlan,
    unit: WorkUnit,
    names: &mut Vec<(i64, String)>,
) -> Result<UnitRowView, ViewError> {
    let agent = agent_name(db, names, assessing_agent(evaluation, &unit)?)?;

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
/// Input: `session`, the session of the window, someone logged in; `id`, the id
/// of an evaluation.
/// Output: the rows (`[{"id": 7, "label": "ADV_ARC.1-1", ...}]`);
/// `ViewError::NotLoggedIn` when nobody is logged in, an evaluation with a
/// cross reference, an error of the database.
///
/// Called by: `ui/src/lib/api/evaluation.ts`: `evaluationUnitList`, from
/// `ui/src/views/evaluation/EvaluationUnits.svelte`, when the tab opens and after each step of an assessment.
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/evaluation/evaluation_plan/evaluation_plan_store.rs`: `read_evaluation_plan`;
/// `objects/evaluation/evaluation_plan/evaluation_plan_ai.rs`: `units_in_order`; [`row_of`].
#[tauri::command(async)]
pub fn evaluation_unit_list(session: State<'_, Session>, id: i64) -> Result<Vec<UnitRowView>, ViewError> {
    let opened = session.lock()?;
    let db = database(opened.as_ref())?;
    let evaluation = read_evaluation_plan(db, id)?;
    let units = units_in_order(db, id)?;
    let mut rows = Vec::with_capacity(units.len());
    let mut names = Vec::new();

    for unit in units {
        rows.push(row_of(db, &evaluation, unit, &mut names)?);
    }

    Ok(rows)
}

/// The names of what the work unit `unit` needs: its dependencies with their
/// verdicts, its documents with their versions, its proofs.
///
/// Input: `db`, the open database; `unit`, a work unit.
/// Output: the names (`Needs { dependencies: ["ADV_ARC.1-1: pass"], .. }`);
/// an error of the database.
///
/// Called by: [`unit_view`].
/// Calls: `objects/evaluation/work_unit/work_unit_store.rs`: `read_work_unit`;
/// `objects/evaluation/product_document/product_document_store.rs`: `read_document_record`;
/// `objects/evaluation/lab_proof/lab_proof_store.rs`: `read_lab_proof`.
fn needs_of(db: &UserDatabase, unit: &WorkUnit) -> Result<Needs, ViewError> {
    let mut dependencies = Vec::with_capacity(unit.dependencies.len());
    let mut documents = Vec::with_capacity(unit.input_product_documents.len());

    for id in &unit.dependencies {
        let dependency = read_work_unit(db, *id)?;
        let detail = dependency.verdict.map(|verdict| verdict_name(verdict).to_owned());

        dependencies.push(NamedView { id: *id, name: dependency.label, detail });
    }

    for id in &unit.input_product_documents {
        let (document, _) = read_document_record(db, *id)?;

        documents.push(NamedView { id: *id, name: document.name, detail: document.version });
    }

    let proofs = match &unit.input_functional {
        Some(ids) => {
            let mut names = Vec::with_capacity(ids.len());

            for id in ids {
                names.push(NamedView { id: *id, name: read_lab_proof(db, *id)?.name, detail: None });
            }

            Some(names)
        }
        None => None,
    };

    Ok(Needs { dependencies, documents, proofs })
}

/// The view of the work unit `unit` of the evaluation `evaluation`.
///
/// Input: `db`, the open database; `evaluation`, the evaluation; `unit`,
/// one of its work units.
/// Output: the work unit whole with what it needs and the last question and
/// answer of the AI; an error of the agent or the database.
///
/// Called by: [`evaluation_unit_get`].
/// Calls: [`needs_of`]; `objects/evaluation/work_unit/work_unit_ai/assessment_question.rs`: `assessing_agent`;
/// `objects/ai/agent/agent_store.rs`: `read_agent`;
/// `objects/evaluation/oscal_standard_document/oscal_standard_document_store.rs`: `read_oscal_standard_document`;
/// `objects/ai/journal/journal_store.rs`: `read_latest_journal_entry`; [`ending_name`], [`author_name`].
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
    let mut proof_quotes = Vec::with_capacity(unit.proof_quotes.len());
    let mut verdicts = Vec::with_capacity(evaluation.possible_verdicts.len());

    for quote in unit.quotes {
        quotes.push(QuoteView { document: quote.product_document, text: quote.text });
    }

    for quote in unit.proof_quotes {
        proof_quotes.push(ProofQuoteView { proof: quote.lab_proof, text: quote.text });
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
        proof_quotes,
        verdict: unit.verdict.map(verdict_name),
        author: unit.author.map(author_name),
        verdicts,
        journal,
    })
}

/// The work unit `unit` of the evaluation `id` whole.
///
/// Input: `session`, the session of the window, someone logged in; `id`, the id
/// of an evaluation; `unit`, the id of one of its work units.
/// Output: the work unit whole; `ViewError::NotLoggedIn` when nobody is logged
/// in, `ViewError::Invalid` for a work unit not of the evaluation, an error of
/// the database.
///
/// Called by: `ui/src/lib/api/evaluation.ts`: `evaluationUnitGet`, from
/// `ui/src/views/evaluation/EvaluationUnit.svelte`, when a work unit is opened.
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/evaluation/evaluation_plan/evaluation_plan_store.rs`: `read_evaluation_plan`;
/// `objects/evaluation/work_unit/work_unit_store.rs`: `read_work_unit`; [`unit_view`].
#[tauri::command(async)]
pub fn evaluation_unit_get(session: State<'_, Session>, id: i64, unit: i64) -> Result<UnitView, ViewError> {
    let opened = session.lock()?;
    let db = database(opened.as_ref())?;
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
/// Input: `name`, the name of a verdict the page sends (`"pass"`).
/// Output: the verdict (`Some(Verdict::Pass)`), `None` for an empty name;
/// `ViewError::Invalid` for another name.
///
/// Called by: [`evaluation_unit_save`].
fn verdict_of(name: &str) -> Result<Option<Verdict>, ViewError> {
    match name {
        "" => Ok(None),
        "pass" => Ok(Some(Verdict::Pass)),
        "fail" => Ok(Some(Verdict::Fail)),
        "inconclusive" => Ok(Some(Verdict::Inconclusive)),
        "not_applicable" => Ok(Some(Verdict::NotApplicable)),
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
    proof_quotes: Vec<ProofQuoteView>,
    /// `"pass"`, `"fail"`, `"inconclusive"`, `"not_applicable"`, or empty for none.
    verdict: String,
}

/// Saves the results of the work unit `unit` of the evaluation `id` as the
/// evaluator gives them (author User). Refused while a work of the AI runs:
/// the evaluator and the AI never write at once (user, 2026-10-30).
///
/// Input: `session`, the session of the window, someone logged in; `id`, the id
/// of an evaluation in progress; `unit`, the id of one of its work units;
/// `fields`, the results as typed.
/// Output: nothing; `ViewError::NotLoggedIn` when nobody is logged in,
/// `ViewError::Invalid` while a work of the AI runs or for a verdict the page
/// never sends, the refusals of the results.
///
/// Called by: `ui/src/lib/api/evaluation.ts`: `evaluationUnitSave`, from
/// `ui/src/views/evaluation/EvaluationUnit.svelte`, "Save".
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`; `panels/session.rs`:
/// `Session::is_working`; [`verdict_of`]; `objects/evaluation/work_unit/work_unit_rules/results.rs`:
/// `save_results`.
#[tauri::command(async)]
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

    for quote in &fields.proof_quotes {
        size += quote.text.len();
    }

    ensure!(size <= MAX_RESULTS_BYTES, ViewError::Invalid("the results are too long".to_owned()));

    ensure!(!session.is_working()?, ViewError::Invalid("the AI is working: save when it has ended".to_owned()));

    let mut quotes = Vec::with_capacity(fields.quotes.len());

    for quote in fields.quotes {
        quotes.push(Quote { product_document: quote.document, text: quote.text });
    }

    let mut proof_quotes = Vec::with_capacity(fields.proof_quotes.len());

    for quote in fields.proof_quotes {
        proof_quotes.push(ProofQuote { lab_proof: quote.proof, text: quote.text });
    }

    let results = Results {
        findings: fields.findings,
        analysis: fields.analysis,
        vulnerability_notes: fields.vulnerability_notes,
        notes: fields.notes,
        quotes,
        proof_quotes,
        verdict: verdict_of(&fields.verdict)?,
    };
    let opened = session.lock()?;

    Ok(save_results(database(opened.as_ref())?, id, unit, &results)?)
}
