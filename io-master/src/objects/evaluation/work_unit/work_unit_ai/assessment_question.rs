//! The question of the assessment of one work unit of an evaluation (user,
//! 2026-10-30), asked of its agent, or of the agent of the evaluation when
//! it has none of its own.
//!
//! Every text of the question is a template of the evaluator role of the
//! agent (`EvaluatorRole`); the program fills it with the parts below.
//!
//! ```text
//! the message of the agent (objects/ai/agent/agent_ai.rs: agent_question):
//!   its instructions, its knowledge found with knowledge_search (RAG, optional),
//!   the task: task_template with {task} (of the documentation, or of a functional test: a work unit with
//!     proofs), {judgement} and {form} (the form of the answer the program reads)
//! the question: documentation_question or functional_question with
//!   {work_unit}     the work unit as JSON: its requirement, guidance and parameters, its ids written as names;
//!                   never its results, so that an assessment made again is not led by the one before
//!   {dependencies}  each by dependency_template: its label, verdict and findings, for the model to weigh
//!   {documents}     each whole by document_template, or the passages found with document_search, each by
//!                   document_passage_template
//!   {proofs}        each by proof_template: its text with the record of the test
//!   {label}, {standard}, {requirement}, {guidance}
//! ```
//!
//! Whether the documents fit whole into the context of the model is told by
//! `objects/ai/integration/integration_ai/question.rs` (`prepare`) when the question is prepared: the caller asks
//! first for
//! them whole, then for their passages.

use crate::objects::ai_error::AiError;
use serde_json::{Value, json};

use crate::infrastructure::clock::date_of;
use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::ai::agent::agent_ai::{agent_question, filled};
use crate::objects::ai::agent::agent_store::read_agent;
use crate::objects::ai::agent_roles::agent_roles_model::EvaluatorRole;
use crate::objects::ai::agent_roles::agent_roles_rules::evaluator_of;
use crate::objects::ai::chat::chat_model::{Message, Speaker};
use crate::objects::ai::integration::integration_ai::question::Question;
use crate::objects::evaluation::evaluation_plan::evaluation_plan_model::EvaluationPlan;
use crate::objects::evaluation::evaluation_plan::evaluation_plan_store::read_evaluation_plan;
use crate::objects::evaluation::lab_proof::lab_proof_store::read_lab_proof;
use crate::objects::evaluation::oscal_standard_document::oscal_standard_document_store::read_oscal_standard_document;
use crate::objects::evaluation::product_document::product_document_store::{
    read_document_passages, read_document_record, read_document_text,
};
use crate::objects::evaluation::work_unit::work_unit_model::{Verdict, WorkUnit};
use crate::objects::evaluation::work_unit::work_unit_store::read_work_unit;

/// The form of the answer the program reads (user, 2026-11-02): only the
/// fields of its JSON and their order, the verdict last; what each field holds
/// is told by the evaluator role of the agent (`EvaluatorRole::judgement`).
/// `{verdicts}` is replaced by the verdicts of the plan. It is given to the
/// agent as `{form}` of its `EvaluatorRole::task_template`.
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
/// Called by: [`unit_json`], [`dependencies_text`], [`task_of`];
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

/// The work unit `unit` as JSON, but its id, author, agent and cross
/// reference, every id written as a name (user, 2026-10-30).
///
/// Input: `unit`, the work unit assessed; `dependencies`, its dependencies,
/// in their order.
/// Output: the JSON without the results of the work unit (`{"label":
/// "ADV_ARC.1-1", "standard": "CEM", "dependencies": [...], ...}`); an error
/// of the database for a standard, a document or a proof missing.
///
/// Called by: [`question_of`].
/// Calls: `objects/evaluation/oscal_standard_document/oscal_standard_document_store.rs`:
/// `read_oscal_standard_document`; `objects/evaluation/product_document/product_document_store.rs`:
/// `read_document_record`; `objects/evaluation/lab_proof/lab_proof_store.rs`: `read_lab_proof`.
fn unit_json(db: &UserDatabase, unit: &WorkUnit, dependencies: &[WorkUnit]) -> Result<Value, AiError> {
    let mut labels = Vec::with_capacity(dependencies.len());
    let mut documents = Vec::with_capacity(unit.input_product_documents.len());

    for dependency in dependencies {
        labels.push(dependency.label.as_str());
    }

    for id in &unit.input_product_documents {
        documents.push(read_document_record(db, *id)?.0.name);
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
        "dependencies": labels,
        "input_product_documents": documents,
        "input_functional": proofs,
    }))
}

/// The dependencies of a work unit, each written by the
/// `dependency_template` of the agent: its label, its verdict and its
/// findings (at most `limit` characters, the agent's).
///
/// Input: `dependencies`, the dependencies of the work unit assessed, in
/// their order; `role`, the evaluator role of its agent; `limit`,
/// `EvaluatorRole::dependency_findings`.
/// Output: the text ("ADV_ARC.1-1: pass\n..."); `EvaluatorRole::nothing_text`
/// without dependencies; a dependency not assessed has
/// `EvaluatorRole::not_assessed_text` for its verdict.
///
/// Called by: [`question_of`].
/// Calls: [`verdict_name`]; `objects/ai/agent/agent_ai.rs`: `filled`.
fn dependencies_text(dependencies: &[WorkUnit], role: &EvaluatorRole, limit: usize) -> String {
    let mut written = Vec::with_capacity(dependencies.len());

    for dependency in dependencies {
        let verdict = match dependency.verdict {
            Some(verdict) => verdict_name(verdict),
            None => role.not_assessed_text.as_str(),
        };
        let findings: String = dependency.findings.chars().take(limit).collect();
        let values = [("label", dependency.label.as_str()), ("verdict", verdict), ("findings", findings.as_str())];

        written.push(filled(&role.dependency_template, &values));
    }

    if written.is_empty() { role.nothing_text.clone() } else { written.join("\n\n") }
}

/// The documents of the work unit `unit`: each whole with `whole`, by the
/// `document_template` of the agent, otherwise the passages of them all
/// found with its `document_search`, by its `document_passage_template`.
///
/// Input: `unit`, the work unit assessed; `role`, the evaluator role of its
/// agent; `whole`, whether the documents go whole; `passages`,
/// `EvaluatorRole::document_passages`, the most passages when they do not.
/// Output: the text ("=== ST ===\n(the version recorded by the evaluator:
/// 1.2)\n..."); `EvaluatorRole::nothing_text` without documents; an error of
/// the database.
///
/// Called by: [`question_of`].
/// Calls: `objects/evaluation/product_document/product_document_store.rs`: `read_document_record`,
/// `read_document_text`, `read_document_passages`; [`search_values`]; `objects/ai/agent/agent_ai.rs`:
/// `filled`.
fn documents_text(
    db: &UserDatabase,
    unit: &WorkUnit,
    role: &EvaluatorRole,
    whole: bool,
    passages: usize,
) -> Result<String, AiError> {
    let mut written = Vec::with_capacity(unit.input_product_documents.len());

    if whole {
        for id in &unit.input_product_documents {
            let (document, _) = read_document_record(db, *id)?;
            let values = [
                ("name", document.name),
                ("version", document.version.unwrap_or_default()),
                ("text", read_document_text(db, *id)?),
            ];

            written.push(filled(&role.document_template, &values));
        }
    } else {
        let wanted = filled(&role.document_search, &search_values(unit));

        for (id, passage) in read_document_passages(db, &unit.input_product_documents, &wanted, passages)? {
            let values = [("name", read_document_record(db, id)?.0.name), ("text", passage)];

            written.push(filled(&role.document_passage_template, &values));
        }
    }

    Ok(if written.is_empty() { role.nothing_text.clone() } else { written.join("\n\n") })
}

/// The placeholders of a search of the work unit `unit`: `{label}`,
/// `{requirement}`, `{guidance}`.
///
/// Called by: [`documents_text`], [`question_of`].
fn search_values(unit: &WorkUnit) -> [(&str, &str); 3] {
    [("label", &unit.label), ("requirement", &unit.requirement), ("guidance", &unit.guidance)]
}

/// The proofs of the work unit `unit`, each with the record of its test, by
/// the `proof_template` of the agent.
///
/// Input: `unit`, the work unit assessed; `role`, the evaluator role of its
/// agent.
/// Output: the text ("=== Test 3 (performed 2026-10-30 by K. K. with ...)
/// ===\n..."); `EvaluatorRole::nothing_text` without proofs;
/// `AiError::Refused` for a proof not given yet, which has no day of its
/// test (the model is never told a day the evaluator did not record), an
/// error of the database for a proof missing.
///
/// Called by: [`question_of`].
/// Calls: `objects/evaluation/lab_proof/lab_proof_store.rs`: `read_lab_proof`; `infrastructure/clock.rs`: `date_of`;
/// `objects/ai/agent/agent_ai.rs`: `filled`.
fn proofs_text(db: &UserDatabase, unit: &WorkUnit, role: &EvaluatorRole) -> Result<String, AiError> {
    let ids = unit.input_functional.as_deref().unwrap_or_default();
    let mut written = Vec::with_capacity(ids.len());

    for id in ids {
        let proof = read_lab_proof(db, *id)?;
        let Some(performed) = proof.performed else {
            return Err(AiError::Refused(format!("the proof {} is not given: it has no day of its test", proof.name)));
        };
        let values = [
            ("name", proof.name),
            ("date", date_of(performed)),
            ("tester", proof.tester.unwrap_or_default()),
            ("equipment", proof.equipment.unwrap_or_default()),
            ("text", proof.proof.unwrap_or_default()),
        ];

        written.push(filled(&role.proof_template, &values));
    }

    Ok(if written.is_empty() { role.nothing_text.clone() } else { written.join("\n\n") })
}

/// The task the agent is told for the work unit `unit`: the
/// `task_template` of its role with the task of the kind of the work unit
/// (of the documentation, or of a functional test when it has proofs), the
/// judgement and the form of the answer with the verdicts of the evaluation.
///
/// Input: `evaluation`, the evaluation of the work unit; `unit`, the work
/// unit assessed; `role`, the evaluator role of its agent.
/// Output: the task ("You assess one work unit ... Answer with one JSON
/// object ...").
///
/// Called by: [`question_of`].
/// Calls: [`verdict_name`]; `objects/ai/agent/agent_ai.rs`: `filled`.
fn task_of(evaluation: &EvaluationPlan, unit: &WorkUnit, role: &EvaluatorRole) -> String {
    let mut verdicts = Vec::with_capacity(evaluation.possible_verdicts.len());

    for verdict in &evaluation.possible_verdicts {
        verdicts.push(format!("\"{}\"", verdict_name(*verdict)));
    }

    // A work unit with proofs is a functional test; one without asks only for
    // its documents (user, 2026-11-02).
    let kind = if unit.input_functional.is_some() { &role.functional_task } else { &role.documentation_task };
    let form = ANSWER_FORM.replace("{verdicts}", &verdicts.join(", "));

    filled(
        &role.task_template,
        &[("task", kind.as_str()), ("judgement", role.judgement.as_str()), ("form", form.as_str())],
    )
}

/// The question about the work unit `unit` of the evaluation `evaluation`,
/// asked of the agent `agent`.
///
/// Input: `evaluation`, an evaluation in progress; `unit`, one of its work
/// units; `agent`, the agent that assesses it; `whole`, whether its documents
/// go whole.
/// Output: the question with the task of its kind from the evaluator role of
/// the agent ([`task_of`]), and the work unit with its evidence written by
/// the question of its kind (`documentation_question`, or
/// `functional_question` when the work unit has proofs); `AiError::Domain`
/// for an agent that is no evaluator, an error of the database or of the
/// agent.
///
/// Called by: [`assessment_question`].
/// Calls: `objects/ai/agent/agent_store.rs`: `read_agent`; `objects/ai/agent_roles/agent_roles_rules.rs`:
/// `evaluator_of`; `objects/evaluation/work_unit/work_unit_store.rs`: `read_work_unit` (each dependency);
/// `objects/evaluation/oscal_standard_document/oscal_standard_document_store.rs`: `read_oscal_standard_document`;
/// [`unit_json`], [`dependencies_text`], [`documents_text`], [`proofs_text`], [`task_of`],
/// [`search_values`]; `objects/ai/agent/agent_ai.rs`: `filled`, `agent_question`.
fn question_of(
    db: &UserDatabase,
    evaluation: &EvaluationPlan,
    unit: &WorkUnit,
    agent: i64,
    whole: bool,
) -> Result<Question, AiError> {
    let stored = read_agent(db, agent)?;
    let role = evaluator_of(&stored.role, &stored.name)?;
    let findings = usize::try_from(role.dependency_findings).map_err(|error| AiError::Refused(error.to_string()))?;
    let passages = usize::try_from(role.document_passages).map_err(|error| AiError::Refused(error.to_string()))?;
    // Only the dependencies of the work unit are read, never every work
    // unit of the evaluation: an assessment asks once for each of them.
    let mut dependencies = Vec::with_capacity(unit.dependencies.len());

    for id in &unit.dependencies {
        dependencies.push(read_work_unit(db, *id)?);
    }

    let values = [
        ("work_unit", unit_json(db, unit, &dependencies)?.to_string()),
        ("label", unit.label.clone()),
        ("standard", read_oscal_standard_document(db, unit.reference)?.title),
        ("requirement", unit.requirement.clone()),
        ("guidance", unit.guidance.clone()),
        ("dependencies", dependencies_text(&dependencies, role, findings)),
        ("documents", documents_text(db, unit, role, whole, passages)?),
        ("proofs", proofs_text(db, unit, role)?),
    ];
    let template =
        if unit.input_functional.is_some() { &role.functional_question } else { &role.documentation_question };
    let text = filled(template, &values);
    let search = filled(&role.knowledge_search, &search_values(unit));
    let messages = vec![Message { speaker: Speaker::User, text, time: 0 }];

    agent_question(db, agent, &task_of(evaluation, unit, role), &search, Some(unit.id), messages)
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
    use crate::objects::evaluation::lab_proof::lab_proof_model::LabProof;
    use crate::objects::evaluation::lab_proof::lab_proof_rules::give_proof;
    use crate::objects::evaluation::work_unit::work_unit_model::Author;
    use crate::objects::evaluation::work_unit::work_unit_store::{create_work_unit, read_work_units};
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
        let functional = read_work_units(&db, needing).unwrap().remove(0);
        let scan = functional.input_functional.unwrap()[0];
        let functional = functional.id;

        // A proof not given has no day of its test: the model is not told one.
        assert!(matches!(assessment_question(&db, needing, functional, true), Err(AiError::Refused(_))));

        let given = LabProof {
            id: scan,
            name: String::new(),
            proof: Some("port 22 closed".to_owned()),
            performed: Some(1_793_000_000),
            tester: Some("K. K.".to_owned()),
            equipment: Some("nmap 7.95".to_owned()),
        };

        give_proof(&db, needing, &given).unwrap();

        let question = assessment_question(&db, needing, functional, true).unwrap();

        assert!(question.system.contains("the proofs of the laboratory show"));
        assert!(
            question.messages[0]
                .text
                .contains("Its proofs of the laboratory:\n=== Scan (performed 2026-10-26 by K. K. with nmap 7.95) ===")
        );

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
}
