//! The rows of the evaluations and the types of `model/evaluation.rs` they
//! become.

use crate::db::error::{Corrupt, DbError};
use crate::model::ai::{Speaker, Turn};
use crate::model::evaluation::{
    Evaluation, EvaluationFields, EvaluationSummary, Evidence, EvidenceKind, Fragment, LabTask, OutputText, Related,
    Results, TaskState, Verdict,
};
use crate::model::task_scheme::{Block, BlockFields, BlockKind, Connection, OutputCondition, OutputFormat};

/// The character that joins the pages of a piece of evidence.
pub(super) const PAGE_BREAK: char = '\u{c}';

/// A value of a column of `table` the program never writes.
fn corrupt(table: &'static str, column: &'static str, value: String) -> DbError {
    DbError::Corrupt(Corrupt { table, column, value })
}

/// A count as a number of the program.
fn count(row: &rusqlite::Row<'_>, column: &str) -> Result<usize, DbError> {
    let value: i64 = row.get(column)?;

    usize::try_from(value).map_err(|error| DbError::Inconsistent(error.to_string()))
}

/// A row of `lab_evaluations` with its counts.
///
/// Called by: `read::evaluations`.
pub(super) fn summary(row: &rusqlite::Row<'_>) -> Result<EvaluationSummary, DbError> {
    Ok(EvaluationSummary {
        uuid: row.get("uuid")?,
        project_title: row.get("project_title")?,
        package_name: row.get("package_name")?,
        product: row.get("product")?,
        tasks: count(row, "tasks")?,
        done: count(row, "done")?,
    })
}

/// A row of `lab_evaluations`.
///
/// Called by: `read::evaluation`.
pub(super) fn evaluation(row: &rusqlite::Row<'_>) -> Result<Evaluation, DbError> {
    Ok(Evaluation {
        uuid: row.get("uuid")?,
        package_uuid: row.get("package_uuid")?,
        project_title: row.get("project_title")?,
        scope: row.get("scope")?,
        package_name: row.get("package_name")?,
        evaluator: row.get("evaluator")?,
        note: row.get("note")?,
        fields: EvaluationFields { product: row.get("product")?, product_version: row.get("product_version")? },
    })
}

/// A row of `lab_tasks`.
///
/// Called by: `read::tasks`, `read::task`.
pub(super) fn task(row: &rusqlite::Row<'_>) -> Result<LabTask, DbError> {
    let state: String = row.get("state")?;

    let Some(state) = TaskState::from_name(&state) else {
        return Err(corrupt("lab_tasks", "state", state));
    };

    Ok(LabTask {
        uuid: row.get("uuid")?,
        catalog_uuid: row.get("catalog_uuid")?,
        control_id: row.get("control_id")?,
        label: row.get("label")?,
        title: row.get("title")?,
        state,
    })
}

/// A row of `lab_blocks`.
///
/// Called by: `read::blocks`, `read::block`.
pub(super) fn block(row: &rusqlite::Row<'_>) -> Result<Block, DbError> {
    let kind: String = row.get("kind")?;
    let format: String = row.get("format")?;
    let condition: String = row.get("condition")?;

    let Some(kind) = BlockKind::from_name(&kind) else {
        return Err(corrupt("lab_blocks", "kind", kind));
    };
    let Some(format) = OutputFormat::from_name(&format) else {
        return Err(corrupt("lab_blocks", "format", format));
    };
    let Some(condition) = OutputCondition::from_name(&condition) else {
        return Err(corrupt("lab_blocks", "condition", condition));
    };

    Ok(Block {
        uuid: row.get("uuid")?,
        kind,
        x: row.get("x")?,
        y: row.get("y")?,
        fields: BlockFields {
            name: row.get("name")?,
            evidence: row.get("evidence")?,
            instructions: row.get("instructions")?,
            target: row.get("target")?,
            format,
            condition,
        },
        proposal: None,
    })
}

/// A row of `lab_connections`.
///
/// Called by: `read::connections`.
pub(super) fn connection(row: &rusqlite::Row<'_>) -> Result<Connection, DbError> {
    Ok(Connection { from: row.get("from_block")?, to: row.get("to_block")? })
}

/// A row of `lab_related`.
///
/// Called by: `read::related`.
pub(super) fn related(row: &rusqlite::Row<'_>) -> Result<Related, DbError> {
    Ok(Related {
        catalog_uuid: row.get("catalog_uuid")?,
        control_id: row.get("control_id")?,
        label: row.get("label")?,
        title: row.get("title")?,
    })
}

/// A row of `lab_evidence`, without its bytes.
///
/// Called by: `read::evidence_of`, `read::evidence`.
pub(super) fn evidence(row: &rusqlite::Row<'_>) -> Result<Evidence, DbError> {
    let kind: String = row.get("kind")?;
    let pages: String = row.get("pages")?;

    let Some(kind) = EvidenceKind::from_name(&kind) else {
        return Err(corrupt("lab_evidence", "kind", kind));
    };

    Ok(Evidence {
        uuid: row.get("uuid")?,
        block: row.get("block_uuid")?,
        kind,
        name: row.get("name")?,
        pages: pages.split(PAGE_BREAK).map(str::to_owned).collect(),
        added: row.get("added")?,
    })
}

/// A row of `lab_messages`.
///
/// Called by: `read::messages`.
pub(super) fn message(row: &rusqlite::Row<'_>) -> Result<Turn, DbError> {
    let speaker: String = row.get("speaker")?;

    let speaker = match speaker.as_str() {
        "evaluator" => Speaker::Evaluator,
        "model" => Speaker::Model,
        _ => return Err(corrupt("lab_messages", "speaker", speaker)),
    };

    Ok(Turn { speaker, text: row.get("text")? })
}

/// The name of `speaker` in `lab_messages`.
///
/// Called by: `create::message`.
pub(super) fn speaker_name(speaker: Speaker) -> &'static str {
    match speaker {
        Speaker::Evaluator => "evaluator",
        Speaker::Model => "model",
    }
}

/// A row of `lab_fragments`.
///
/// Called by: `read::fragments`, `read::accepted_fragments`.
pub(super) fn fragment(row: &rusqlite::Row<'_>) -> Result<Fragment, DbError> {
    let page: Option<i64> = row.get("page")?;

    let page = match page {
        Some(page) => Some(u32::try_from(page).map_err(|error| DbError::Inconsistent(error.to_string()))?),
        None => None,
    };

    Ok(Fragment {
        uuid: row.get("uuid")?,
        evidence: row.get("evidence_uuid")?,
        page,
        quote: row.get("quote")?,
        note: row.get("note")?,
        found: row.get("found")?,
        accepted: row.get("accepted")?,
        by_ai: row.get("by_ai")?,
    })
}

/// A row of `lab_results` or `lab_result_proposals` (named `table`).
///
/// Called by: `read::results`, `read::results_proposal`.
pub(super) fn results(row: &rusqlite::Row<'_>, table: &'static str) -> Result<Results, DbError> {
    let verdict: Option<String> = row.get("verdict")?;

    let verdict = match verdict {
        Some(name) => match Verdict::from_name(&name) {
            Some(verdict) => Some(verdict),
            None => return Err(corrupt(table, "verdict", name)),
        },
        None => None,
    };

    Ok(Results {
        findings: row.get("findings")?,
        analysis: row.get("analysis")?,
        assessment: row.get("assessment")?,
        vulnerabilities: row.get("vulnerabilities")?,
        verdict,
    })
}

/// A row of `lab_outputs`.
///
/// Called by: `read::output`.
pub(super) fn output(row: &rusqlite::Row<'_>) -> Result<OutputText, DbError> {
    Ok(OutputText { block: row.get("block_uuid")?, text: row.get("text")?, proposal: row.get("proposal")? })
}
