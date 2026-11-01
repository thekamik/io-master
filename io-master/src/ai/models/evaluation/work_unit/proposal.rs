//! "Fill with AI" on one work unit (user, 2026-10-29): the agent of the
//! planner proposes its parameters; they are written at once, over what was
//! there, as if the planner had written them. Two tasks, each started by
//! the planner on its own (the loop over the work units of the plan:
//! `ai/models/evaluation/evaluation_plan.rs`: `fill_plan`; one step:
//! `filling.rs`):
//!
//! ```text
//! the parameters (every one but the dependencies, which stay as they are):
//!   parameters_question ──► the work unit, its parameters now, the documents, the agents
//!   read_parameters     ──► ParametersProposal
//!   apply_parameters    ──► only the documents and agents there are ──► update_work_unit_parameters
//!
//! the dependencies (when the work units are filled, so each is described):
//!   dependencies_question ──► the work unit, its dependencies now, every other work unit
//!                             with the first sentence of its requirement and its kind of test
//!   read_dependencies     ──► the labels
//!   apply_dependencies    ──► only the work units of the plan ──► update_work_unit_parameters
//! ```
//!
//! Every question also carries the instructions of the agent, its task and
//! the knowledge of the agent that fits the work unit; it is asked through
//! the integration of the agent and kept in the journal of the agent. **A dependency that
//! closes a cycle is written as proposed** (user, 2026-10-29): the planner
//! decides; the work units of the cycle are marked `cross_reference`, the
//! plan `blocked`, until the planner corrects them.

use crate::ai::error::AiError;
use std::collections::HashMap;

use serde_json::{Value, json};

use crate::ai::models::ai::agent::agent_question;
use crate::ai::models::ai::integration::question::Question;
use crate::db::models::ai::agent::read_agents;
use crate::db::models::evaluation::lab_proof::{create_lab_proof, read_lab_proof};
use crate::db::models::evaluation::oscal_standard_document::read_oscal_standard_document;
use crate::db::models::evaluation::product_document::read_product_document;
use crate::db::models::evaluation::work_unit::{
    MAX_DEPENDENCIES, MAX_DOCUMENTS, MAX_PROOFS, read_work_unit, read_work_units,
};
use crate::db::storage::user_database::UserDatabase;
use crate::domain::models::evaluation::evaluation_plan::planning::plan_to_fill;
use crate::domain::models::evaluation::work_unit::parameters::{WorkUnitParameters, update_work_unit_parameters};
use crate::model::ai::chat::{Message, Speaker};
use crate::model::evaluation::evaluation_plan::EvaluationPlan;
use crate::model::evaluation::lab_proof::LabProof;
use crate::model::evaluation::work_unit::WorkUnit;

/// The longest guidance taken from an answer, in characters.
const MAX_GUIDANCE: usize = 20_000;

/// The longest name of a proof taken from an answer, in characters.
const MAX_PROOF_NAME: usize = 200;

/// The longest description of another work unit in a question of the
/// dependencies, in characters.
const MAX_SUMMARY: usize = 200;

/// The name of the proof of a functional test the answer names none for.
const DEFAULT_PROOF: &str = "Results of the functional test";

/// The task of the parameters, given after the instructions of the agent.
const PARAMETERS_TASK: &str = "You plan one work unit of a security evaluation. From its requirement, its parameters \
now and your knowledge, propose its parameters. Answer with one JSON object and nothing else:\n\
{\"documents\": [the names of the documents of the product it needs],\n \
\"functional\": true when it needs a functional test of the laboratory, false for a test of the documentation,\n \
\"proofs\": [the names of the proofs of the laboratory its functional test needs],\n \
\"agent\": the name of the agent that should assess it, or null,\n \
\"guidance\": the guidance for the evaluator, whole}\n\
Use only the documents and agents the question lists.";

/// The task of the dependencies, given after the instructions of the agent.
const DEPENDENCIES_TASK: &str = "You plan the order of the work of a security evaluation. From the requirement of one \
work unit, the other work units of the plan and your knowledge, name the work units whose results this one needs, so \
that they are done before it. Answer with one JSON object and nothing else:\n\
{\"dependencies\": [the labels of those work units]}\n\
Use only the labels the question lists; an empty list when it needs none.";

/// The parameters but the dependencies an answer proposes, as names,
/// before they are checked against the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParametersProposal {
    pub documents: Vec<String>,
    pub functional: bool,
    pub proofs: Vec<String>,
    pub agent: Option<String>,
    pub guidance: String,
}

/// The names a work unit of a plan may name, by their ids.
struct Names {
    /// The documents of the product of the plan.
    documents: HashMap<i64, String>,
    /// Every agent of the user.
    agents: HashMap<i64, String>,
}

/// The label of each work unit of `units`, by its id.
///
/// Called by: [`dependencies_question`], [`apply_dependencies`].
fn labels(units: &[WorkUnit]) -> HashMap<i64, String> {
    let mut labels = HashMap::with_capacity(units.len());

    for unit in units {
        labels.insert(unit.id, unit.label.clone());
    }

    labels
}

/// The names of the things `ids` name, by `names`; an id without a name
/// is left out.
///
/// Called by: [`current_parameters`], [`parameters_question`],
/// [`dependencies_question`].
fn named(ids: &[i64], names: &HashMap<i64, String>) -> Vec<String> {
    let mut found = Vec::with_capacity(ids.len());

    for id in ids {
        if let Some(name) = names.get(id) {
            found.push(name.clone());
        }
    }

    found
}

/// The names of the documents of the plan and of every agent, by their
/// ids.
///
/// Called by: [`parameters_question`], [`apply_parameters`].
/// Calls: `read_product_document`, `read_agents`.
fn names_of(db: &UserDatabase, plan: &EvaluationPlan) -> Result<Names, AiError> {
    let mut documents = HashMap::with_capacity(plan.product_documents.len());
    let mut agents = HashMap::new();

    for id in &plan.product_documents {
        documents.insert(*id, read_product_document(db, *id)?.name);
    }

    for agent in read_agents(db)? {
        agents.insert(agent.id, agent.name);
    }

    Ok(Names { documents, agents })
}

/// The question of the agent of the planner of the plan `plan` about its
/// work unit `unit`: its instructions and the task `task`, with its
/// knowledge that fits the work unit, and the text `text` of the question.
///
/// Called by: [`parameters_question`], [`dependencies_question`].
/// Calls: `domain/models/evaluation/evaluation_plan/planning.rs`: `plan_to_fill`, `ai/models/ai/agent.rs`: `agent_question`.
fn question_of(db: &UserDatabase, plan: i64, unit: &WorkUnit, task: &str, text: String) -> Result<Question, AiError> {
    let (stored, agent_id) = plan_to_fill(db, plan)?;

    ensure!(stored.work_units.contains(&unit.id), AiError::Refused("the work unit is not one of the plan".to_owned()));

    let search = format!("{} {}", unit.label, unit.requirement);
    let messages = vec![Message { speaker: Speaker::User, text, time: 0 }];

    agent_question(db, agent_id, task, &search, Some(unit.id), messages)
}

/// The parameters but the dependencies of the work unit `unit` now, as the
/// question shows them (names, not ids).
///
/// Called by: [`parameters_question`].
/// Calls: [`named`], `read_lab_proof`.
fn current_parameters(db: &UserDatabase, unit: &WorkUnit, names: &Names) -> Result<Value, AiError> {
    let mut proofs = Vec::new();

    for id in unit.input_functional.as_deref().unwrap_or_default() {
        proofs.push(read_lab_proof(db, *id)?.name);
    }

    let agent = match unit.ai_agent {
        Some(id) => names.agents.get(&id).cloned(),
        None => None,
    };

    Ok(json!({
        "documents": named(&unit.input_product_documents, &names.documents),
        "functional": unit.input_functional.is_some(),
        "proofs": proofs,
        "agent": agent,
        "guidance": unit.guidance,
    }))
}

/// The question of the parameters of the work unit `unit` of the plan
/// `plan`: its requirement, its parameters now, the documents of the plan
/// and the agents.
///
/// Called by: `filling.rs`: `prepared_question`.
/// Calls: `domain/models/evaluation/evaluation_plan/planning.rs`: `plan_to_fill`, [`names_of`], [`current_parameters`],
/// [`question_of`].
pub fn parameters_question(db: &UserDatabase, plan: i64, unit: i64) -> Result<Question, AiError> {
    let (stored, _) = plan_to_fill(db, plan)?;
    let this = read_work_unit(db, unit)?;
    let names = names_of(db, &stored)?;
    let mut agents: Vec<String> = names.agents.values().cloned().collect();

    agents.sort();

    let text = format!(
        "Work unit: {}\nStandard: {}\n\nRequirement:\n{}\n\nIts parameters now:\n{}\n\nThe documents of the product: \
         {}\nThe agents: {}",
        this.label,
        read_oscal_standard_document(db, this.reference)?.title,
        this.requirement,
        current_parameters(db, &this, &names)?,
        named(&stored.product_documents, &names.documents).join(", "),
        agents.join(", "),
    );

    question_of(db, plan, &this, PARAMETERS_TASK, text)
}

/// One line about a work unit for the question of the dependencies: its
/// label, the first sentence of its requirement (at most [`MAX_SUMMARY`]
/// characters) and its kind of test.
///
/// Called by: [`dependencies_question`].
fn summary(unit: &WorkUnit) -> String {
    let requirement = unit.requirement.trim();
    let first = match requirement.split_once(". ") {
        Some((sentence, _)) => sentence,
        None => requirement,
    };
    let short: String = first.chars().take(MAX_SUMMARY).collect();
    let kind = if unit.input_functional.is_some() { "functional" } else { "documentation" };

    format!("{} ({kind}): {short}", unit.label)
}

/// The question of the dependencies of the work unit `unit` of the plan
/// `plan`: its requirement, its dependencies now, and every other work unit
/// of the plan in one line.
///
/// Called by: `filling.rs`: `prepared_question`.
/// Calls: [`labels`], [`named`], [`summary`], [`question_of`].
pub fn dependencies_question(db: &UserDatabase, plan: i64, unit: i64) -> Result<Question, AiError> {
    let units = read_work_units(db, plan)?;
    let this = read_work_unit(db, unit)?;
    let mut others = Vec::with_capacity(units.len());

    for other in &units {
        if other.id != unit {
            others.push(summary(other));
        }
    }

    let text = format!(
        "Work unit: {}\n\nRequirement:\n{}\n\nIts dependencies now: {}\n\nThe other work units of the plan:\n{}",
        this.label,
        this.requirement,
        named(&this.dependencies, &labels(&units)).join(", "),
        others.join("\n"),
    );

    question_of(db, plan, &this, DEPENDENCIES_TASK, text)
}

/// The JSON object of an answer (a fence of Markdown or words around it
/// are passed over); refused when the answer holds none.
///
/// Called by: [`read_parameters`], [`read_dependencies`].
fn answer_object(answer: &str) -> Result<Value, AiError> {
    let (Some(start), Some(end)) = (answer.find('{'), answer.rfind('}')) else {
        return Err(AiError::Refused("the answer holds no JSON object".to_owned()));
    };

    serde_json::from_str(answer.get(start..=end).unwrap_or_default())
        .map_err(|error| AiError::Refused(format!("the answer is no JSON object: {error}")))
}

/// The strings of the list `field` of the answer, at most `max`; what is no
/// string is left out.
///
/// Called by: [`read_parameters`], [`read_dependencies`].
fn strings(answer: &Value, field: &str, max: usize) -> Vec<String> {
    let mut found = Vec::new();

    if let Some(items) = answer.get(field).and_then(Value::as_array) {
        for item in items.iter().take(max) {
            if let Some(text) = item.as_str() {
                found.push(text.trim().to_owned());
            }
        }
    }

    found
}

/// The proposal of the parameters in an answer.
///
/// Called by: `filling.rs`: `applied`.
/// Calls: [`answer_object`], [`strings`].
pub fn read_parameters(answer: &str) -> Result<ParametersProposal, AiError> {
    let value = answer_object(answer)?;
    let guidance = value.get("guidance").and_then(Value::as_str).unwrap_or_default().trim();

    ensure!(
        guidance.chars().count() <= MAX_GUIDANCE,
        AiError::Refused(format!("a guidance over {MAX_GUIDANCE} characters"))
    );

    Ok(ParametersProposal {
        documents: strings(&value, "documents", MAX_DOCUMENTS),
        functional: value.get("functional").and_then(Value::as_bool).unwrap_or(false),
        proofs: strings(&value, "proofs", MAX_PROOFS),
        agent: value.get("agent").and_then(Value::as_str).map(str::trim).map(str::to_owned),
        guidance: guidance.to_owned(),
    })
}

/// The labels of the dependencies an answer proposes.
///
/// Called by: `filling.rs`: `applied`.
/// Calls: [`answer_object`], [`strings`].
pub fn read_dependencies(answer: &str) -> Result<Vec<String>, AiError> {
    Ok(strings(&answer_object(answer)?, "dependencies", MAX_DEPENDENCIES))
}

/// The id of the name `name` among `names`, its case and the spaces around
/// it passed over.
///
/// Called by: [`apply_parameters`], [`apply_dependencies`].
fn id_of(names: &HashMap<i64, String>, name: &str) -> Option<i64> {
    let wanted = name.trim().to_lowercase();

    for (id, known) in names {
        if known.trim().to_lowercase() == wanted {
            return Some(*id);
        }
    }

    None
}

/// The proof of the work unit `unit` named `name` (its case passed over),
/// if it has one.
///
/// Called by: [`chosen_proofs`].
/// Calls: `read_lab_proof`.
fn proof_named(db: &UserDatabase, unit: &WorkUnit, name: &str) -> Result<Option<i64>, AiError> {
    for id in unit.input_functional.as_deref().unwrap_or_default() {
        if read_lab_proof(db, *id)?.name.eq_ignore_ascii_case(name) {
            return Ok(Some(*id));
        }
    }

    Ok(None)
}

/// The proofs of a functional test: a proof of the work unit `unit` with
/// the same name is kept, a new name gets a new place.
///
/// Called by: [`apply_parameters`].
/// Calls: [`proof_named`], `create_lab_proof`.
fn chosen_proofs(db: &UserDatabase, unit: &WorkUnit, proposed: &[String]) -> Result<Vec<i64>, AiError> {
    let mut names = Vec::with_capacity(proposed.len());

    for name in proposed {
        if !name.is_empty() && name.chars().count() <= MAX_PROOF_NAME && !names.contains(name) {
            names.push(name.clone());
        }
    }

    if names.is_empty() {
        names.push(DEFAULT_PROOF.to_owned());
    }

    let mut proofs = Vec::with_capacity(names.len());

    for name in &names {
        let id = match proof_named(db, unit, name)? {
            Some(kept) => kept,
            None => create_lab_proof(
                db,
                &LabProof { id: 0, name: name.clone(), proof: None, performed: None, tester: None, equipment: None },
            )?,
        };

        proofs.push(id);
    }

    Ok(proofs)
}

/// The agent the proposal names, among `agents`; `None` for none, with a
/// note for a name that is no agent.
///
/// Called by: [`apply_parameters`].
/// Calls: [`id_of`].
fn chosen_agent(agents: &HashMap<i64, String>, proposed: Option<&String>, notes: &mut Vec<String>) -> Option<i64> {
    let name = proposed?;

    if name.is_empty() {
        return None;
    }

    let found = id_of(agents, name);

    if found.is_none() {
        notes.push(format!("no agent {name}"));
    }

    found
}

/// Writes the proposed parameters of the work unit `unit` of the plan
/// `plan`, its dependencies kept: only the documents and agents there are.
/// Gives a note for every name left out.
///
/// Called by: `filling.rs`: `applied`.
/// Calls: `domain/models/evaluation/evaluation_plan/planning.rs`: `plan_to_fill`, [`names_of`], [`id_of`], [`chosen_agent`],
/// [`chosen_proofs`], `update_work_unit_parameters`.
pub fn apply_parameters(
    db: &UserDatabase,
    plan: i64,
    unit: i64,
    proposal: &ParametersProposal,
) -> Result<Vec<String>, AiError> {
    let (stored, _) = plan_to_fill(db, plan)?;
    let this = read_work_unit(db, unit)?;
    let names = names_of(db, &stored)?;
    let mut notes = Vec::new();
    let mut documents = Vec::with_capacity(proposal.documents.len());

    for name in &proposal.documents {
        match id_of(&names.documents, name) {
            Some(id) if !documents.contains(&id) => documents.push(id),
            Some(_) => {}
            None => notes.push(format!("no document {name} in the plan")),
        }
    }

    let parameters = WorkUnitParameters {
        dependencies: this.dependencies.clone(),
        input_product_documents: documents,
        input_functional: if proposal.functional { chosen_proofs(db, &this, &proposal.proofs)? } else { Vec::new() },
        ai_agent: chosen_agent(&names.agents, proposal.agent.as_ref(), &mut notes),
        guidance: proposal.guidance.clone(),
    };

    update_work_unit_parameters(db, plan, unit, &parameters)?;

    Ok(notes)
}

/// Writes the proposed dependencies of the work unit `unit` of the plan
/// `plan`, its other parameters kept: only the work units of the plan. A
/// dependency that closes a cycle is written; the planner decides, and a
/// note says that the work unit is now a cross reference. Gives a note for
/// every label left out.
///
/// Called by: `filling.rs`: `applied`.
/// Calls: `domain/models/evaluation/evaluation_plan/planning.rs`: `plan_to_fill`, [`labels`], [`id_of`],
/// `update_work_unit_parameters`.
pub fn apply_dependencies(
    db: &UserDatabase,
    plan: i64,
    unit: i64,
    proposed: &[String],
) -> Result<Vec<String>, AiError> {
    plan_to_fill(db, plan)?;

    let units = read_work_units(db, plan)?;
    let this = read_work_unit(db, unit)?;
    let names = labels(&units);
    let mut notes = Vec::new();
    let mut dependencies = Vec::with_capacity(proposed.len());

    for label in proposed {
        match id_of(&names, label) {
            None => notes.push(format!("no work unit {label} in the plan")),
            Some(id) if id == unit => notes.push("a dependency on itself left out".to_owned()),
            Some(id) if !dependencies.contains(&id) => dependencies.push(id),
            Some(_) => {}
        }
    }

    let parameters = WorkUnitParameters {
        dependencies,
        input_product_documents: this.input_product_documents.clone(),
        input_functional: this.input_functional.clone().unwrap_or_default(),
        ai_agent: this.ai_agent,
        guidance: this.guidance.clone(),
    };

    update_work_unit_parameters(db, plan, unit, &parameters)?;

    if read_work_unit(db, unit)?.cross_reference {
        notes.push("closes a cycle of dependencies: correct it in the editor of the work unit".to_owned());
    }

    Ok(notes)
}

#[cfg(test)]
mod tests {
    //! Tests of the filling: the two questions, the reading of the answers,
    //! the writing of the proposals; a cycle left to the planner.

    use super::*;

    use crate::db::models::evaluation::evaluation_plan::read_evaluation_plan;
    use crate::test_support::database::database;
    use crate::test_support::evaluation::plan_with_planner;

    #[test]
    fn the_two_questions_show_what_their_task_needs() {
        let (_directory, db) = database();
        let (plan, units) = plan_with_planner(&db);
        let parameters = parameters_question(&db, plan, units[0]).unwrap();
        let dependencies = dependencies_question(&db, plan, units[0]).unwrap();
        let text = &parameters.messages[0].text;
        let others = &dependencies.messages[0].text;

        assert_eq!((parameters.agent.is_some(), parameters.work_unit), (true, Some(units[0])));
        assert!(
            parameters.system.contains("You assess work units of the CEM.") && parameters.system.contains("guidance")
        );
        assert!(text.contains("Work unit: 1.1.1") && text.contains("The documents of the product: ST"));
        assert!(text.contains("The agents: CC evaluator") && !text.contains("dependencies"));
        assert!(dependencies.system.contains("\"dependencies\""));
        assert!(others.contains("1.1.2 (documentation): Conflicting duties") && !others.contains("1.1.1 ("));
    }

    #[test]
    fn the_answers_are_read_from_their_json_objects() {
        let answer = "Here it is:\n```json\n{\"documents\": [\"ST\", 7], \"functional\": true, \"proofs\": [\"Scan\"], \
                      \"agent\": null, \"guidance\": \" Read. \"}\n```";
        let proposal = read_parameters(answer).unwrap();

        assert_eq!(proposal.documents, vec!["ST"]);
        assert_eq!((proposal.functional, proposal.agent, proposal.guidance.as_str()), (true, None, "Read."));
        assert_eq!(read_dependencies("{\"dependencies\": [\"1.1.1\"]}").unwrap(), vec!["1.1.1"]);
        assert!(matches!(read_parameters("I cannot."), Err(AiError::Refused(_))));
    }

    #[test]
    fn the_parameters_keep_the_dependencies_and_a_proof_of_the_same_name() {
        let (_directory, db) = database();
        let (plan, units) = plan_with_planner(&db);
        let proposal = ParametersProposal {
            documents: vec!["st".to_owned(), "IXIT".to_owned()],
            functional: true,
            proofs: vec!["Scan of the ports".to_owned()],
            agent: Some("CC evaluator".to_owned()),
            guidance: "Check the roles.".to_owned(),
        };

        apply_dependencies(&db, plan, units[3], &["1.1.2".to_owned()]).unwrap();

        assert_eq!(apply_parameters(&db, plan, units[3], &proposal).unwrap(), vec!["no document IXIT in the plan"]);

        let filled = read_work_unit(&db, units[3]).unwrap();

        assert_eq!((filled.dependencies.clone(), filled.guidance.as_str()), (vec![units[1]], "Check the roles."));
        assert!(filled.ai_agent.is_some() && filled.input_functional.as_ref().map(Vec::len) == Some(1));

        apply_parameters(&db, plan, units[3], &proposal).unwrap();
        assert_eq!(read_work_unit(&db, units[3]).unwrap().input_functional, filled.input_functional);
    }

    #[test]
    fn a_dependency_that_closes_a_cycle_is_written_and_left_to_the_planner() {
        let (_directory, db) = database();
        let (plan, units) = plan_with_planner(&db);
        let proposal = ParametersProposal {
            documents: Vec::new(),
            functional: false,
            proofs: Vec::new(),
            agent: None,
            guidance: "Kept.".to_owned(),
        };

        apply_parameters(&db, plan, units[0], &proposal).unwrap();

        assert!(apply_dependencies(&db, plan, units[0], &["1.1.2".to_owned()]).unwrap().is_empty());

        let notes = apply_dependencies(&db, plan, units[1], &["1.1.1".to_owned(), "9.9".to_owned()]).unwrap();

        assert!(notes.iter().any(|note| note.contains("no work unit 9.9")));
        assert!(notes.iter().any(|note| note.contains("closes a cycle")));
        assert!(read_evaluation_plan(&db, plan).unwrap().blocked);
        assert!(read_work_unit(&db, units[0]).unwrap().cross_reference);
        assert_eq!(read_work_unit(&db, units[0]).unwrap().guidance, "Kept.");
    }
}
