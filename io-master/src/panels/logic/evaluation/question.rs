//! The question of the assessment of one work unit of an evaluation (user,
//! 2026-10-30), asked of its agent, or of the agent of the evaluation when
//! it has none of its own.
//!
//! ```text
//! the instructions of the agent + the task (the verdicts of the plan, the form of the answer)
//!   + the knowledge of the agent that fits the work unit (RAG)
//! the question:
//!   the work unit as JSON, but id, author, ai_agent and cross_reference; its ids written as names
//!   its dependencies: their labels, verdicts and findings
//!   its documents: their text whole, or the passages that fit the work unit best
//!   its proofs: their text with the record of the test
//! ```
//!
//! Whether the documents fit whole into the context of the model is told by
//! `ai/ask.rs` when the question is prepared: the caller asks first for
//! them whole, then for their passages.

use serde_json::{Value, json};

use crate::ai::ask::Question;
use crate::db::ai::agent::read_agent;
use crate::db::database::Database;
use crate::db::evaluation::evaluation_plan::read_evaluation_plan;
use crate::db::evaluation::lab_proof::read_lab_proof;
use crate::db::evaluation::oscal_standard_document::read_oscal_standard_document;
use crate::db::evaluation::product_document::{read_document_passages, read_document_text, read_product_document};
use crate::db::evaluation::work_unit::{read_work_unit, read_work_units};
use crate::model::ai::chat::{Message, Speaker};
use crate::model::evaluation::evaluation_plan::EvaluationPlan;
use crate::model::evaluation::work_unit::{Verdict, WorkUnit};
use crate::panels::logic::agents::agent::agent_knowledge;
use crate::panels::logic::evaluation::error::EvaluationError;

/// The most passages of the documents given with a question when the
/// documents do not fit whole.
const DOCUMENT_PASSAGES: usize = 20;

/// The longest findings of a dependency given with a question, in
/// characters.
const MAX_DEPENDENCY_FINDINGS: usize = 1000;

/// The task of the assessment, given after the instructions of the agent;
/// `{verdicts}` is replaced by the verdicts of the plan.
const ASSESSMENT_TASK: &str = "You assess one work unit of a security evaluation: whether the documents of the \
product and the proofs of the laboratory given with it meet its requirement. Answer with one JSON object and nothing \
else:\n\
{\"verdict\": one of {verdicts},\n \
\"findings\": every fault you found, one per line, or an empty text,\n \
\"analysis\": how the evidence meets or fails the requirement,\n \
\"vulnerability_notes\": what an attacker could use, or an empty text,\n \
\"notes\": anything else the evaluator should know,\n \
\"quotes\": [{\"document\": the name of a document given, \"text\": a fragment of it, word for word}]}\n\
Rest every statement on a quote. Quote only the documents given, word for word: a quote that is not found in its \
document is not taken.";

/// The name of a verdict, as the question and the answer write it.
///
/// Called by: [`unit_json`], [`dependencies_text`], [`question_of`];
/// `answer.rs`.
pub fn verdict_name(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Pass => "pass",
        Verdict::Fail => "fail",
        Verdict::Inconclusive => "inconclusive",
    }
}

/// The agent that assesses the work unit `unit`: its own, or the agent of
/// the evaluation; refused when there is neither.
///
/// Called by: [`assessment_question`]; `run.rs` (the preview).
pub fn assessing_agent(evaluation: &EvaluationPlan, unit: &WorkUnit) -> Result<i64, EvaluationError> {
    match unit.ai_agent.or(evaluation.ai_agent) {
        Some(agent) => Ok(agent),
        None => Err(EvaluationError::Refused(format!("{} has no agent, nor has the evaluation", unit.label))),
    }
}

/// The label of the work unit `id` among `units`.
///
/// Called by: [`unit_json`], [`dependencies_text`].
fn label_of(units: &[WorkUnit], id: i64) -> String {
    for unit in units {
        if unit.id == id {
            return unit.label.clone();
        }
    }

    format!("work unit {id}")
}

/// The work unit `unit` as JSON, but its id, author, agent and cross
/// reference, every id written as a name (user, 2026-10-30).
///
/// Called by: [`question_of`].
/// Calls: `read_oscal_standard_document`, [`label_of`],
/// `read_product_document`, `read_lab_proof`, [`verdict_name`].
fn unit_json(db: &Database, unit: &WorkUnit, units: &[WorkUnit]) -> Result<Value, EvaluationError> {
    let mut dependencies = Vec::with_capacity(unit.dependencies.len());
    let mut documents = Vec::with_capacity(unit.input_product_documents.len());
    let mut quotes = Vec::with_capacity(unit.quotes.len());

    for id in &unit.dependencies {
        dependencies.push(label_of(units, *id));
    }

    for id in &unit.input_product_documents {
        documents.push(read_product_document(db, *id)?.name);
    }

    for quote in &unit.quotes {
        quotes.push(json!({"document": read_product_document(db, quote.product_document)?.name, "text": quote.text}));
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

    Ok(json!({
        "standard": read_oscal_standard_document(db, unit.reference)?.title,
        "label": unit.label,
        "oscal_key": unit.oscal_key,
        "requirement": unit.requirement,
        "guidance": unit.guidance,
        "dependencies": dependencies,
        "input_product_documents": documents,
        "input_functional": proofs,
        "findings": unit.findings,
        "analysis": unit.analysis,
        "vulnerability_notes": unit.vulnerability_notes,
        "notes": unit.notes,
        "quotes": quotes,
        "verdict": unit.verdict.map(verdict_name),
    }))
}

/// The dependencies of the work unit `unit`, one paragraph each: its label,
/// its verdict and its findings (at most [`MAX_DEPENDENCY_FINDINGS`]
/// characters).
///
/// Called by: [`question_of`].
/// Calls: [`label_of`], [`verdict_name`].
fn dependencies_text(unit: &WorkUnit, units: &[WorkUnit]) -> String {
    let mut text = String::new();

    for id in &unit.dependencies {
        for other in units {
            if other.id == *id {
                let verdict = other.verdict.map_or("not assessed", verdict_name);
                let findings: String = other.findings.chars().take(MAX_DEPENDENCY_FINDINGS).collect();

                text.push_str(&format!("{}: {verdict}\n{findings}\n\n", label_of(units, *id)));
            }
        }
    }

    if text.is_empty() { "none\n".to_owned() } else { text }
}

/// The documents of the work unit `unit`: each whole with `whole`, otherwise
/// the passages of them all that fit the work unit best, each under the name
/// and the version of its document.
///
/// Called by: [`question_of`].
/// Calls: `read_product_document`, `read_document_text`,
/// `read_document_passages`.
fn documents_text(db: &Database, unit: &WorkUnit, whole: bool) -> Result<String, EvaluationError> {
    let mut text = String::new();

    if whole {
        for id in &unit.input_product_documents {
            let document = read_product_document(db, *id)?;
            let version = document.version.unwrap_or_default();

            text.push_str(&format!(
                "=== {} (version {version}) ===\n{}\n",
                document.name,
                read_document_text(db, *id)?
            ));
        }
    } else {
        let wanted = format!("{} {}", unit.requirement, unit.guidance);

        for (id, passage) in read_document_passages(db, &unit.input_product_documents, &wanted, DOCUMENT_PASSAGES)? {
            text.push_str(&format!("=== a passage of {} ===\n{passage}\n\n", read_product_document(db, id)?.name));
        }
    }

    Ok(if text.is_empty() { "none\n".to_owned() } else { text })
}

/// The day of `seconds` since 1970 (UTC), as YYYY-MM-DD (the civil calendar
/// of the days since 1970, H. Hinnant).
///
/// Called by: [`proofs_text`].
fn day_of(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400) + 719_468;
    let era = days.div_euclid(146_097);
    let of_era = days - era * 146_097;
    let year_of_era = (of_era - of_era / 1460 + of_era / 36_524 - of_era / 146_096) / 365;
    let of_year = of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * of_year + 2) / 153;
    let day = of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 { month_index + 3 } else { month_index - 9 };
    let year = year_of_era + era * 400 + i64::from(month <= 2);

    format!("{year:04}-{month:02}-{day:02}")
}

/// The proofs of the work unit `unit`, each with the record of its test.
///
/// Called by: [`question_of`].
/// Calls: `read_lab_proof`, [`day_of`].
fn proofs_text(db: &Database, unit: &WorkUnit) -> Result<String, EvaluationError> {
    let mut text = String::new();

    for id in unit.input_functional.as_deref().unwrap_or_default() {
        let proof = read_lab_proof(db, *id)?;

        text.push_str(&format!(
            "=== {} (performed {} by {} with {}) ===\n{}\n\n",
            proof.name,
            day_of(proof.performed.unwrap_or_default()),
            proof.tester.unwrap_or_default(),
            proof.equipment.unwrap_or_default(),
            proof.proof.unwrap_or_default(),
        ));
    }

    Ok(if text.is_empty() { "none\n".to_owned() } else { text })
}

/// The question about the work unit `unit` of the evaluation `evaluation`,
/// asked of the agent `agent`; its documents whole with `whole`.
///
/// Called by: [`assessment_question`].
/// Calls: `read_agent`, `agent_knowledge`, [`unit_json`],
/// [`dependencies_text`], [`documents_text`], [`proofs_text`].
fn question_of(
    db: &Database,
    evaluation: &EvaluationPlan,
    unit: &WorkUnit,
    agent: i64,
    whole: bool,
) -> Result<Question, EvaluationError> {
    let units = read_work_units(db, evaluation.id)?;
    let stored = read_agent(db, agent)?;
    let mut verdicts = Vec::with_capacity(evaluation.possible_verdicts.len());

    for verdict in &evaluation.possible_verdicts {
        verdicts.push(format!("\"{}\"", verdict_name(*verdict)));
    }

    let mut system =
        format!("{}\n\n{}", stored.instructions, ASSESSMENT_TASK.replace("{verdicts}", &verdicts.join(", ")));
    let knowledge = agent_knowledge(db, agent, &format!("{} {}", unit.label, unit.requirement))
        .map_err(|error| EvaluationError::Refused(format!("the knowledge of the agent: {error}")))?;

    if !knowledge.is_empty() {
        system.push_str("\n\nYour knowledge for this work unit:\n\n");
        system.push_str(&knowledge.join("\n\n---\n\n"));
    }

    let text = format!(
        "The work unit:\n{}\n\nIts dependencies:\n{}\nIts documents of the product:\n{}\nIts proofs of the laboratory:\n{}",
        unit_json(db, unit, &units)?,
        dependencies_text(unit, &units),
        documents_text(db, unit, whole)?,
        proofs_text(db, unit)?,
    );

    Ok(Question {
        integration: stored.integration,
        agent: Some(agent),
        work_unit: Some(unit.id),
        system,
        messages: vec![Message { speaker: Speaker::User, text, time: 0 }],
    })
}

/// The question of the assessment of the work unit `unit` of the evaluation
/// `evaluation`, its documents whole with `whole`, otherwise their passages
/// that fit it best. Refused when the work unit is not one of the
/// evaluation or has no agent.
///
/// Called by: `run.rs` (each work unit).
/// Calls: `read_evaluation_plan`, `read_work_unit`, [`assessing_agent`],
/// [`question_of`].
pub fn assessment_question(
    db: &Database,
    evaluation: i64,
    unit: i64,
    whole: bool,
) -> Result<Question, EvaluationError> {
    let stored = read_evaluation_plan(db, evaluation)?;
    let this = read_work_unit(db, unit)?;

    ensure!(
        stored.work_units.contains(&unit),
        EvaluationError::Refused("the work unit is not one of the evaluation".to_owned())
    );

    let agent = assessing_agent(&stored, &this)?;

    question_of(db, &stored, &this, agent, whole)
}

#[cfg(test)]
mod tests {
    //! Tests of the question: the work unit as JSON without what it must not
    //! carry, its dependencies with their verdicts, its documents whole or by
    //! passages.

    use super::*;

    use crate::model::evaluation::work_unit::Author;
    use crate::test_support::database::database;
    use crate::test_support::evaluation::{evaluation, write};

    #[test]
    fn the_question_carries_the_work_unit_its_dependencies_and_its_documents() {
        let (_directory, db) = database();
        let (id, first, second) = evaluation(&db);
        let assessed = WorkUnit {
            findings: "The ST names no TOE.".to_owned(),
            verdict: Some(Verdict::Fail),
            author: Some(Author::Ai),
            ..read_work_unit(&db, first).unwrap()
        };

        write(&db, &assessed);

        let question = assessment_question(&db, id, second, true).unwrap();
        let text = &question.messages[0].text;

        assert_eq!(
            (question.work_unit, question.agent),
            (Some(second), read_evaluation_plan(&db, id).unwrap().ai_agent)
        );
        assert!(question.system.contains("one of \"pass\", \"fail\""));
        assert!(text.contains("\"label\":\"ASE_INT.1-2\"") && text.contains("\"dependencies\":[\"ASE_INT.1-1\"]"));
        assert!(text.contains("\"input_product_documents\":[\"ST\"]") && text.contains("\"standard\":\"CEM\""));
        assert!(!text.contains("\"id\"") && !text.contains("\"author\"") && !text.contains("cross_reference"));
        assert!(text.contains("ASE_INT.1-1: fail\nThe ST names no TOE."));
        assert!(text.contains("=== ST (version 1.0) ===") && text.contains("Its TSF filters packets."));

        let by_passages = assessment_question(&db, id, second, false).unwrap();

        assert!(by_passages.messages[0].text.contains("=== a passage of ST ==="));
        assert!(matches!(assessment_question(&db, id, 999, true), Err(EvaluationError::Db(_))));
    }

    #[test]
    fn a_day_is_written_from_its_seconds() {
        assert_eq!(day_of(0), "1970-01-01");
        assert_eq!(day_of(1_793_000_000), "2026-10-26");
        assert_eq!(day_of(951_782_400), "2000-02-29");
    }
}
