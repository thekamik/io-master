//! The question of the assessment of one work unit of an evaluation (user,
//! 2026-10-30), asked of its agent, or of the agent of the evaluation when
//! it has none of its own.
//!
//! ```text
//! the instructions of the agent
//!   + the knowledge of the agent that fits the work unit (RAG, optional), each passage under its source
//!   + the task: of the documentation, or of a functional test (a work unit with proofs); the form of the answer
//! the question:
//!   the work unit as JSON: its requirement, guidance and parameters, its ids written as names; never its
//!     results, so that an assessment made again is not led by the one before
//!   its dependencies: their labels, verdicts and findings, for the model to weigh
//!   its documents: their text whole, or the passages that fit the work unit best
//!   its proofs (a functional test only): their text with the record of the test
//! ```
//!
//! Whether the documents fit whole into the context of the model is told by
//! `objects/ai/integration/integration_ai/question.rs` (`prepare`) when the question is prepared: the caller asks
//! first for
//! them whole, then for their passages.

use crate::objects::ai_error::AiError;
use serde_json::{Value, json};

use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::ai::agent::agent_ai::agent_question;
use crate::objects::ai::agent::agent_store::read_agent;
use crate::objects::ai::agent_roles::agent_roles_rules::evaluator_of;
use crate::objects::ai::chat::chat_model::{Message, Speaker};
use crate::objects::ai::integration::integration_ai::question::Question;
use crate::objects::evaluation::evaluation_plan::evaluation_plan_model::EvaluationPlan;
use crate::objects::evaluation::evaluation_plan::evaluation_plan_store::read_evaluation_plan;
use crate::objects::evaluation::lab_proof::lab_proof_store::read_lab_proof;
use crate::objects::evaluation::oscal_standard_document::oscal_standard_document_store::read_oscal_standard_document;
use crate::objects::evaluation::product_document::product_document_store::{
    read_document_passages, read_document_text, read_product_document,
};
use crate::objects::evaluation::work_unit::work_unit_model::{Verdict, WorkUnit};
use crate::objects::evaluation::work_unit::work_unit_store::{read_work_unit, read_work_units};

/// The form of the answer the program reads (user, 2026-11-02): only the
/// fields of its JSON and their order, the verdict last; what each field holds
/// is told by the evaluator role of the agent (`EvaluatorRole::judgement`).
/// `{verdicts}` is replaced by the verdicts of the plan.
const ANSWER_FORM: &str = "Answer with one JSON object and nothing else, its fields in this order:\n\
{\"findings\": text, \"analysis\": text, \"vulnerability_notes\": text, \"notes\": text,\n \
\"quotes\": [{\"document\": the name of a document or a proof given, \"text\": text}],\n \
\"verdict\": one of {verdicts}}";

/// The name of a verdict, as the question and the answer write it:
/// `Verdict::Pass` is "pass".
///
/// Input: `verdict`, a verdict.
/// Output: its name (`"pass"`).
///
/// Called by: [`unit_json`], [`dependencies_text`], [`question_of`];
/// `objects/evaluation/work_unit/work_unit_ai/assessment_answer.rs`: `verdict_of`;
/// `panels/views/evaluation/work_units.rs`: `row_of`, `needs_of`,
/// `unit_view`; `panels/views/progress.rs`: `progress_view`.
pub fn verdict_name(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Pass => "pass",
        Verdict::Fail => "fail",
        Verdict::Inconclusive => "inconclusive",
        Verdict::NotApplicable => "not_applicable",
    }
}

/// The agent that assesses the work unit `unit`: its own, or the agent of
/// the evaluation.
///
/// Input: `evaluation`, the evaluation of the work unit; `unit`, one of its
/// work units.
/// Output: the id of the agent (`3`); `AiError::Refused` when there is
/// neither.
///
/// Called by: [`assessment_question`]; `objects/evaluation/evaluation_plan/evaluation_plan_ai.rs`:
/// `preview`; `panels/views/evaluation/work_units.rs`: `row_of`, `unit_view`.
pub fn assessing_agent(evaluation: &EvaluationPlan, unit: &WorkUnit) -> Result<i64, AiError> {
    match unit.ai_agent.or(evaluation.ai_agent) {
        Some(agent) => Ok(agent),
        None => Err(AiError::Refused(format!("{} has no agent, nor has the evaluation", unit.label))),
    }
}

/// The label of the work unit `id` among `units`.
///
/// Input: `units`, the work units of the evaluation; `id`, the id of one of
/// them.
/// Output: its label ("ADV_ARC.1-1"); "work unit 12" when none of `units`
/// has the id.
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
/// Input: `unit`, the work unit assessed; `units`, the work units of its
/// evaluation.
/// Output: the JSON without the results of the work unit (`{"label":
/// "ADV_ARC.1-1", "standard": "CEM", "dependencies": [...], ...}`); an error
/// of the database for a standard, a document or a proof missing.
///
/// Called by: [`question_of`].
/// Calls: `objects/evaluation/oscal_standard_document/oscal_standard_document_store.rs`:
/// `read_oscal_standard_document`; [`label_of`]; `objects/evaluation/product_document/product_document_store.rs`:
/// `read_product_document`; `objects/evaluation/lab_proof/lab_proof_store.rs`: `read_lab_proof`.
fn unit_json(db: &UserDatabase, unit: &WorkUnit, units: &[WorkUnit]) -> Result<Value, AiError> {
    let mut dependencies = Vec::with_capacity(unit.dependencies.len());
    let mut documents = Vec::with_capacity(unit.input_product_documents.len());

    for id in &unit.dependencies {
        dependencies.push(label_of(units, *id));
    }

    for id in &unit.input_product_documents {
        documents.push(read_product_document(db, *id)?.name);
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
    }))
}

/// The dependencies of the work unit `unit`, one paragraph each: its label,
/// its verdict and its findings (at most `limit` characters, the agent's).
///
/// Input: `unit`, the work unit assessed; `units`, the work units of its
/// evaluation; `limit`, `EvaluatorRole::dependency_findings` of its agent.
/// Output: the text ("ADV_ARC.1-1: pass\n...\n\n"); empty without
/// dependencies.
///
/// Called by: [`question_of`].
/// Calls: [`label_of`], [`verdict_name`].
fn dependencies_text(unit: &WorkUnit, units: &[WorkUnit], limit: usize) -> String {
    let mut text = String::new();

    for id in &unit.dependencies {
        for other in units {
            if other.id == *id {
                let verdict = other.verdict.map_or("not assessed", verdict_name);
                let findings: String = other.findings.chars().take(limit).collect();

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
/// Input: `unit`, the work unit assessed; `whole`, whether the documents go
/// whole; `passages`, `EvaluatorRole::document_passages` of its agent, the
/// most passages when they do not.
/// Output: the text ("=== ST ===\n(the version recorded by the evaluator:
/// 1.2)\n..."); an error of the database.
///
/// Called by: [`question_of`].
/// Calls: `objects/evaluation/product_document/product_document_store.rs`: `read_product_document`,
/// `read_document_text`, `read_document_passages`.
fn documents_text(db: &UserDatabase, unit: &WorkUnit, whole: bool, passages: usize) -> Result<String, AiError> {
    let mut text = String::new();

    if whole {
        for id in &unit.input_product_documents {
            let document = read_product_document(db, *id)?;
            let version = document.version.unwrap_or_default();

            // The version is the evaluator's record, apart from the name the
            // quotes give and apart from the text of the document.
            text.push_str(&format!(
                "=== {} ===\n(the version recorded by the evaluator: {version})\n{}\n",
                document.name,
                read_document_text(db, *id)?
            ));
        }
    } else {
        let wanted = format!("{} {}", unit.requirement, unit.guidance);

        for (id, passage) in read_document_passages(db, &unit.input_product_documents, &wanted, passages)? {
            text.push_str(&format!("=== a passage of {} ===\n{passage}\n\n", read_product_document(db, id)?.name));
        }
    }

    Ok(if text.is_empty() { "none\n".to_owned() } else { text })
}

/// The day of `seconds` since 1970 (UTC), as YYYY-MM-DD (the civil calendar
/// of the days since 1970, H. Hinnant): `day_of(1793577600)` is "2026-11-02".
///
/// Input: `seconds`, a time written by the program.
/// Output: its day ("2026-11-02").
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
/// Input: `unit`, the work unit assessed.
/// Output: the text ("=== Test 3 (performed 2026-10-30 by K. K. with ...)
/// ===\n..."); an error of the database for a proof missing.
///
/// Called by: [`question_of`].
/// Calls: `objects/evaluation/lab_proof/lab_proof_store.rs`: `read_lab_proof`; [`day_of`].
fn proofs_text(db: &UserDatabase, unit: &WorkUnit) -> Result<String, AiError> {
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
/// asked of the agent `agent`.
///
/// Input: `evaluation`, an evaluation in progress; `unit`, one of its work
/// units; `agent`, the agent that assesses it; `whole`, whether its documents
/// go whole.
/// Output: the question with the task of its kind from the evaluator role of
/// the agent (of the documentation, or of a functional test when the work
/// unit has proofs), its judgement and the form of the answer, and the work
/// unit with its evidence; `AiError::Domain` for an agent that is no
/// evaluator, an error of the database or of the agent.
///
/// Called by: [`assessment_question`].
/// Calls: `objects/ai/agent/agent_store.rs`: `read_agent`; `objects/ai/agent_roles/agent_roles_rules.rs`:
/// `evaluator_of`; `objects/evaluation/work_unit/work_unit_store.rs`: `read_work_units`; [`unit_json`],
/// [`dependencies_text`], [`documents_text`], [`proofs_text`], [`verdict_name`]; `objects/ai/agent/agent_ai.rs`:
/// `agent_question`.
fn question_of(
    db: &UserDatabase,
    evaluation: &EvaluationPlan,
    unit: &WorkUnit,
    agent: i64,
    whole: bool,
) -> Result<Question, AiError> {
    let units = read_work_units(db, evaluation.id)?;
    let stored = read_agent(db, agent)?;
    let role = evaluator_of(&stored.role, &stored.name)?;
    let findings = usize::try_from(role.dependency_findings).map_err(|error| AiError::Refused(error.to_string()))?;
    let passages = usize::try_from(role.document_passages).map_err(|error| AiError::Refused(error.to_string()))?;
    let mut verdicts = Vec::with_capacity(evaluation.possible_verdicts.len());

    for verdict in &evaluation.possible_verdicts {
        verdicts.push(format!("\"{}\"", verdict_name(*verdict)));
    }

    // A work unit with proofs is a functional test; one without asks only for
    // its documents, and its question names no proofs (user, 2026-11-02).
    let functional = unit.input_functional.is_some();
    let kind = if functional { &role.functional_task } else { &role.documentation_task };
    let task = format!("{kind}\n\n{}\n\n{}", role.judgement, ANSWER_FORM.replace("{verdicts}", &verdicts.join(", ")));
    let mut text = format!(
        "The work unit:\n{}\n\nIts dependencies, with their results:\n{}\nIts documents of the product:\n{}",
        unit_json(db, unit, &units)?,
        dependencies_text(unit, &units, findings),
        documents_text(db, unit, whole, passages)?,
    );

    if functional {
        text.push_str(&format!("\nIts proofs of the laboratory:\n{}", proofs_text(db, unit)?));
    }

    let search = format!("{} {}", unit.label, unit.requirement);
    let messages = vec![Message { speaker: Speaker::User, text, time: 0 }];

    agent_question(db, agent, &task, &search, Some(unit.id), messages)
        .map_err(|error| AiError::Refused(format!("the question of the agent: {error}")))
}

/// The question of the assessment of the work unit `unit` of the evaluation
/// `evaluation`, its documents whole with `whole`, otherwise their passages
/// that fit it best.
///
/// Input: `evaluation`, the id of an evaluation; `unit`, the id of one of its
/// work units; `whole`, whether the documents go whole.
/// Output: the question; `AiError::Refused` when the work unit is not one of
/// the evaluation, names no document and no proof, or has no agent; the
/// errors of [`question_of`].
///
/// Called by: `objects/evaluation/work_unit/work_unit_ai/assessment.rs`: `prepared_assessment`.
/// Calls: `objects/evaluation/evaluation_plan/evaluation_plan_store.rs`: `read_evaluation_plan`;
/// `objects/evaluation/work_unit/work_unit_store.rs`: `read_work_unit`; [`assessing_agent`], [`question_of`].
pub fn assessment_question(db: &UserDatabase, evaluation: i64, unit: i64, whole: bool) -> Result<Question, AiError> {
    let stored = read_evaluation_plan(db, evaluation)?;
    let this = read_work_unit(db, unit)?;

    ensure!(
        stored.work_units.contains(&unit),
        AiError::Refused("the work unit is not one of the evaluation".to_owned())
    );

    // A work unit is assessed on its evidence: one that names no document and
    // no proof would be judged on nothing (user, 2026-11-02).
    ensure!(
        !this.input_product_documents.is_empty()
            || this.input_functional.as_ref().is_some_and(|proofs| !proofs.is_empty()),
        AiError::Refused(format!("{} names no document and no proof to assess", this.label))
    );

    let agent = assessing_agent(&stored, &this)?;

    question_of(db, &stored, &this, agent, whole)
}

#[cfg(test)]
mod tests {
    //! Tests of the question: the work unit as JSON without what it must not
    //! carry (its results among them), its dependencies with their verdicts,
    //! its documents whole or by passages, the task of its kind, and a work
    //! unit without evidence refused.

    use super::*;

    use crate::objects::evaluation::evaluation_plan::evaluation_plan_store::{
        create_evaluation, create_evaluation_plan,
    };
    use crate::objects::evaluation::work_unit::work_unit_model::Author;
    use crate::objects::evaluation::work_unit::work_unit_store::create_work_unit;
    use crate::test_support::database::{database, plan, unit};
    use crate::test_support::evaluation::{evaluation, evaluation_needing_inputs, write};

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

        write(&db, &WorkUnit { analysis: "An earlier analysis.".to_owned(), ..read_work_unit(&db, second).unwrap() });

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
        assert!(text.contains("=== ST ===\n(the version recorded by the evaluator: 1.0)"));
        assert!(text.contains("Its TSF filters packets."));
        assert!(question.system.contains("\"verdict\": one of \"pass\", \"fail\"}"));
        assert!(question.system.contains("Decide the verdict last") && question.system.contains("Judge only what"));
        assert!(!text.contains("An earlier analysis.") && !text.contains("\"verdict\""));
        assert!(question.system.contains("as its documents describe it") && !text.contains("proofs of the laboratory"));

        let by_passages = assessment_question(&db, id, second, false).unwrap();

        assert!(by_passages.messages[0].text.contains("=== a passage of ST ==="));
        assert!(matches!(assessment_question(&db, id, 999, true), Err(AiError::Db(_))));
    }

    #[test]
    fn a_functional_test_is_asked_about_its_proofs_and_a_unit_without_evidence_is_refused() {
        let (_directory, db) = database();
        let needing = evaluation_needing_inputs(&db);
        let functional = read_work_units(&db, needing).unwrap()[0].id;
        let question = assessment_question(&db, needing, functional, true).unwrap();

        assert!(question.system.contains("the proofs of the laboratory show"));
        assert!(question.messages[0].text.contains("Its proofs of the laboratory:\n=== Scan"));

        // A plan whose work unit names no document and no proof, begun as an
        // evaluation: its planning is frozen there.
        let routers = plan(&db);
        let source = create_evaluation_plan(&db, &routers).unwrap();
        let bare = WorkUnit { input_product_documents: Vec::new(), ..unit(&routers, "ASE_INT.1-1") };

        create_work_unit(&db, source, &bare).unwrap();

        let general = read_evaluation_plan(&db, needing).unwrap().ai_agent;
        let empty = create_evaluation(&db, source, "R7", "ACME", "K. K.", 1_793_000_000, general).unwrap();
        let unit_of_empty = read_work_units(&db, empty).unwrap()[0].id;

        assert!(matches!(assessment_question(&db, empty, unit_of_empty, true), Err(AiError::Refused(_))));
    }

    #[test]
    fn a_day_is_written_from_its_seconds() {
        assert_eq!(day_of(0), "1970-01-01");
        assert_eq!(day_of(1_793_000_000), "2026-10-26");
        assert_eq!(day_of(951_782_400), "2000-02-29");
    }
}
