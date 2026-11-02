//! "Fill with AI" on one work unit (user, 2026-10-29): the agent of the
//! planner proposes its parameters; they are written at once, over what was
//! there, as if the planner had written them. Two tasks, each started by
//! the planner on its own (the loop over the work units of the plan:
//! `objects/evaluation/evaluation_plan/evaluation_plan_ai.rs`: `fill_plan`; one step:
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
//! Every text of the two questions is a template of the planner role of the
//! agent (`PlannerRole`: `parameters_question`, `dependencies_question`,
//! `summary_template`, `knowledge_search`), filled by the program. Every
//! question also carries the instructions of the agent, its task and the
//! knowledge of the agent that fits the work unit; it is asked through
//! the integration of the agent and kept in the journal of the agent. **A dependency that
//! closes a cycle is written as proposed** (user, 2026-10-29): the planner
//! decides; the work units of the cycle are marked `cross_reference`, the
//! plan `blocked`, until the planner corrects them.

use crate::objects::ai_error::AiError;
use std::collections::HashMap;

use serde_json::{Value, json};

use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::ai::agent::agent_ai::{agent_question, filled};
use crate::objects::ai::agent::agent_store::{read_agent, read_agents};
use crate::objects::ai::agent_roles::agent_roles_model::{AgentRole, PlannerRole};
use crate::objects::ai::agent_roles::agent_roles_rules::planner_of;
use crate::objects::ai::chat::chat_model::{Message, Speaker};
use crate::objects::ai::integration::integration_ai::question::Question;
use crate::objects::evaluation::evaluation_plan::evaluation_plan_model::EvaluationPlan;
use crate::objects::evaluation::evaluation_plan::evaluation_plan_rules::planning::plan_to_fill;
use crate::objects::evaluation::lab_proof::lab_proof_model::LabProof;
use crate::objects::evaluation::lab_proof::lab_proof_store::{create_lab_proof, read_lab_proof};
use crate::objects::evaluation::oscal_standard_document::oscal_standard_document_store::read_oscal_standard_document;
use crate::objects::evaluation::product_document::product_document_store::read_product_document;
use crate::objects::evaluation::work_unit::work_unit_model::WorkUnit;
use crate::objects::evaluation::work_unit::work_unit_rules::parameters::{
    WorkUnitParameters, update_work_unit_parameters,
};
use crate::objects::evaluation::work_unit::work_unit_store::{
    MAX_DEPENDENCIES, MAX_DOCUMENTS, MAX_PROOFS, read_work_unit, read_work_units,
};

/// The longest guidance taken from an answer, in characters.
const MAX_GUIDANCE: usize = 20_000;

/// The longest name of a proof taken from an answer, in characters.
const MAX_PROOF_NAME: usize = 200;

/// The form of the answer of the parameters the program reads (user,
/// 2026-11-02): only the fields of its JSON; what each holds is told by the
/// planner role of the agent (`PlannerRole::parameters_task`), after which
/// the form comes.
const PARAMETERS_FORM: &str = "Answer with one JSON object and nothing else:\n\
{\"documents\": [text], \"functional\": true or false, \"proofs\": [text], \"agent\": text or null, \
\"guidance\": text}";

/// The form of the answer of the dependencies the program reads.
const DEPENDENCIES_FORM: &str = "Answer with one JSON object and nothing else:\n\
{\"dependencies\": [the labels of the work units]}";

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

/// The label of each work unit of `units`, by its id: `{7: "ADV_ARC.1-1"}`.
///
/// Input: `units`, the work units of a plan.
/// Output: their labels by id.
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
/// Input: `ids`, the ids of documents, agents or work units; `names`, their
/// names by id.
/// Output: the names, in the order of `ids` (`["ST", "IXIT"]`).
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

/// The names of the documents of the plan and of every agent that may assess
/// a work unit (an evaluator), by their ids.
///
/// Input: `plan`, a plan in planning.
/// Output: the names (`Names { documents: {9: "ST"}, agents: {2: "Planner"} }`);
/// an error of the database.
///
/// Called by: [`parameters_question`], [`apply_parameters`].
/// Calls: `objects/evaluation/product_document/product_document_store.rs`: `read_product_document`;
/// `objects/ai/agent/agent_store.rs`: `read_agents`.
fn names_of(db: &UserDatabase, plan: &EvaluationPlan) -> Result<Names, AiError> {
    let mut documents = HashMap::with_capacity(plan.product_documents.len());
    let mut agents = HashMap::new();

    for id in &plan.product_documents {
        documents.insert(*id, read_product_document(db, *id)?.name);
    }

    // Only an evaluator may assess a work unit (user, 2026-11-02).
    for agent in read_agents(db)? {
        if matches!(agent.role, AgentRole::Evaluator(_)) {
            agents.insert(agent.id, agent.name);
        }
    }

    Ok(Names { documents, agents })
}

/// The planner role of the agent of the planner of the plan `plan`: the
/// tasks it asks with and its limits.
///
/// Input: `plan`, the id of a plan to fill.
/// Output: the parameters of its planner (`PlannerRole { summary_characters:
/// 200, .. }`); the refusals of a plan not to fill or an agent that is no
/// planner, an error of the database.
///
/// Called by: [`parameters_question`], [`dependencies_question`], [`apply_parameters`].
/// Calls: `objects/evaluation/evaluation_plan/evaluation_plan_rules/planning.rs`: `plan_to_fill`;
/// `objects/ai/agent/agent_store.rs`: `read_agent`; `objects/ai/agent_roles/agent_roles_rules.rs`: `planner_of`.
fn planner_role(db: &UserDatabase, plan: i64) -> Result<PlannerRole, AiError> {
    let (_, agent) = plan_to_fill(db, plan)?;
    let stored = read_agent(db, agent)?;

    Ok(planner_of(&stored.role, &stored.name)?.clone())
}

/// The question of the agent of the planner of the plan `plan` about its
/// work unit `unit`: its instructions and the task `task`, with its
/// knowledge found by the `knowledge_search` of its role, and the text
/// `text` of the question.
///
/// Input: `plan`, the id of a plan to fill; `unit`, one of its work units;
/// `role`, the planner role of the agent; `task`, the task of its role with
/// the form of the answer; `text`, the question.
/// Output: the question; the refusals of
/// `objects/evaluation/evaluation_plan/evaluation_plan_rules/planning.rs`:
/// `plan_to_fill`, `AiError::Refused` for a work unit not of the plan, the
/// errors of the agent.
///
/// Called by: [`parameters_question`], [`dependencies_question`].
/// Calls: `objects/evaluation/evaluation_plan/evaluation_plan_rules/planning.rs`: `plan_to_fill`;
/// `objects/ai/agent/agent_ai.rs`: `filled`, `agent_question`.
fn question_of(
    db: &UserDatabase,
    plan: i64,
    unit: &WorkUnit,
    role: &PlannerRole,
    task: &str,
    text: String,
) -> Result<Question, AiError> {
    let (stored, agent_id) = plan_to_fill(db, plan)?;

    ensure!(stored.work_units.contains(&unit.id), AiError::Refused("the work unit is not one of the plan".to_owned()));

    let search =
        filled(&role.knowledge_search, &[("label", unit.label.as_str()), ("requirement", unit.requirement.as_str())]);
    let messages = vec![Message { speaker: Speaker::User, text, time: 0 }];

    agent_question(db, agent_id, task, &search, Some(unit.id), messages)
}

/// The parameters but the dependencies of the work unit `unit` now, as the
/// question shows them (names, not ids).
///
/// Input: `unit`, a work unit of the plan; `names`, from [`names_of`].
/// Output: the JSON (`{"documents": ["ST"], "functional": false, ...}`); an
/// error of the database for a proof missing.
///
/// Called by: [`parameters_question`].
/// Calls: [`named`]; `objects/evaluation/lab_proof/lab_proof_store.rs`: `read_lab_proof`.
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
/// `plan`, by the `parameters_question` of the planner: its requirement, its
/// parameters now, the documents of the plan and the agents.
///
/// Input: `plan`, the id of a plan to fill; `unit`, the id of one of its work
/// units.
/// Output: the question; the refusals of [`question_of`], an error of the
/// database.
///
/// Called by: `objects/evaluation/work_unit/work_unit_ai/filling.rs`: `prepared_question`.
/// Calls: `objects/evaluation/evaluation_plan/evaluation_plan_rules/planning.rs`: `plan_to_fill`;
/// `objects/evaluation/work_unit/work_unit_store.rs`: `read_work_unit`;
/// `objects/evaluation/oscal_standard_document/oscal_standard_document_store.rs`: `read_oscal_standard_document`;
/// [`names_of`], [`planner_role`], [`current_parameters`], [`named`], [`question_of`]; `objects/ai/agent/agent_ai.rs`:
/// `filled`.
pub fn parameters_question(db: &UserDatabase, plan: i64, unit: i64) -> Result<Question, AiError> {
    let (stored, _) = plan_to_fill(db, plan)?;
    let this = read_work_unit(db, unit)?;
    let names = names_of(db, &stored)?;
    let role = planner_role(db, plan)?;
    let mut agents: Vec<String> = names.agents.values().cloned().collect();

    agents.sort();

    let values = [
        ("label", this.label.clone()),
        ("standard", read_oscal_standard_document(db, this.reference)?.title),
        ("requirement", this.requirement.clone()),
        ("parameters", current_parameters(db, &this, &names)?.to_string()),
        ("documents", named(&stored.product_documents, &names.documents).join(", ")),
        ("agents", agents.join(", ")),
    ];
    let text = filled(&role.parameters_question, &values);
    let task = format!("{}\n\n{PARAMETERS_FORM}", role.parameters_task);

    question_of(db, plan, &this, &role, &task, text)
}

/// One line about a work unit for the question of the dependencies, by the
/// `summary_template` of the planner: its label, its kind of test and the
/// first sentence of its requirement (at most `limit` characters, the
/// planner's).
///
/// Input: `unit`, a work unit of the plan; `role`, the planner role of the
/// agent; `limit`, `PlannerRole::summary_characters`.
/// Output: the line ("ADV_ARC.1-1 (documentation): The developer shall ...").
///
/// Called by: [`dependencies_question`].
/// Calls: `objects/ai/agent/agent_ai.rs`: `filled`.
fn summary(unit: &WorkUnit, role: &PlannerRole, limit: usize) -> String {
    let requirement = unit.requirement.trim();
    let first = match requirement.split_once(". ") {
        Some((sentence, _)) => sentence,
        None => requirement,
    };
    let short: String = first.chars().take(limit).collect();
    let kind = if unit.input_functional.is_some() { "functional" } else { "documentation" };

    filled(&role.summary_template, &[("label", unit.label.as_str()), ("kind", kind), ("sentence", short.as_str())])
}

/// The question of the dependencies of the work unit `unit` of the plan
/// `plan`, by the `dependencies_question` of the planner: its requirement,
/// its dependencies now, and every other work unit of the plan in one line.
///
/// Input: `plan`, the id of a plan to fill; `unit`, the id of one of its work
/// units.
/// Output: the question; the refusals of [`question_of`], an error of the
/// database.
///
/// Called by: `objects/evaluation/work_unit/work_unit_ai/filling.rs`: `prepared_question`.
/// Calls: [`planner_role`]; `objects/evaluation/work_unit/work_unit_store.rs`: `read_work_units`, `read_work_unit`;
/// [`labels`], [`named`], [`summary`], [`question_of`]; `objects/ai/agent/agent_ai.rs`: `filled`.
pub fn dependencies_question(db: &UserDatabase, plan: i64, unit: i64) -> Result<Question, AiError> {
    let role = planner_role(db, plan)?;
    let limit = usize::try_from(role.summary_characters).map_err(|error| AiError::Refused(error.to_string()))?;
    let units = read_work_units(db, plan)?;
    let this = read_work_unit(db, unit)?;
    let mut others = Vec::with_capacity(units.len());

    for other in &units {
        if other.id != unit {
            others.push(summary(other, &role, limit));
        }
    }

    let values = [
        ("label", this.label.clone()),
        ("requirement", this.requirement.clone()),
        ("dependencies", named(&this.dependencies, &labels(&units)).join(", ")),
        ("others", others.join("\n")),
    ];
    let text = filled(&role.dependencies_question, &values);
    let task = format!("{}\n\n{DEPENDENCIES_FORM}", role.dependencies_task);

    question_of(db, plan, &this, &role, &task, text)
}

/// The JSON object of an answer: a fence of Markdown or words around it are
/// passed over.
///
/// Input: `answer`, the text of the answer of the AI, bounded by
/// `MAX_STREAM` (`connection.rs`).
/// Output: the object (`{"documents": [...], ...}`); `AiError::Refused`
/// when the answer holds none.
///
/// Called by: [`read_parameters`], [`read_dependencies`].
fn answer_object(answer: &str) -> Result<Value, AiError> {
    let (Some(start), Some(end)) = (answer.find('{'), answer.rfind('}')) else {
        return Err(AiError::Refused("the answer holds no JSON object".to_owned()));
    };

    // A "}" before the first "{" leaves no object between them.
    ensure!(start < end, AiError::Refused("the answer holds no JSON object".to_owned()));

    let object =
        answer.get(start..=end).ok_or_else(|| AiError::Refused("the answer holds no JSON object".to_owned()))?;

    serde_json::from_str(object).map_err(|error| AiError::Refused(format!("the answer is no JSON object: {error}")))
}

/// The strings of the list `field` of the answer, at most `max`; what is no
/// string is left out.
///
/// Input: `answer`, the object of the answer; `field`, a list of it
/// ("documents"); `max`, the most kept.
/// Output: the strings trimmed (`["ST", "IXIT"]`); empty without the list.
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

/// The proposal of the parameters in an answer, before it is checked
/// against the plan.
///
/// Input: `answer`, the text of the answer of the AI.
/// Output: the proposal, its lists cut to their limits
/// (`ParametersProposal { documents: ["ST"], functional: true, .. }`); the
/// refusals of [`answer_object`], `AiError::Refused` for a guidance over
/// [`MAX_GUIDANCE`].
///
/// Called by: `objects/evaluation/work_unit/work_unit_ai/filling.rs`: `applied`.
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

/// The labels of the dependencies an answer proposes, at most
/// `MAX_DEPENDENCIES`: `["ADV_ARC.1-1"]`.
///
/// Called by: `objects/evaluation/work_unit/work_unit_ai/filling.rs`: `applied`.
/// Calls: [`answer_object`], [`strings`].
pub fn read_dependencies(answer: &str) -> Result<Vec<String>, AiError> {
    Ok(strings(&answer_object(answer)?, "dependencies", MAX_DEPENDENCIES))
}

/// The id of the name `name` among `names`, its case and the spaces around
/// it passed over.
///
/// Input: `names`, names by id; `name`, a name the answer gives (" st ").
/// Output: its id (`Some(9)`); `None` when no name matches.
///
/// Called by: [`chosen_agent`], [`apply_parameters`], [`apply_dependencies`].
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
/// Input: `unit`, a work unit of the plan; `name`, a name the answer gives.
/// Output: its id (`Some(6)`), `None` when the work unit has no such proof;
/// an error of the database.
///
/// Called by: [`chosen_proofs`].
/// Calls: `objects/evaluation/lab_proof/lab_proof_store.rs`: `read_lab_proof`.
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
/// Input: `unit`, a work unit of the plan; `proposed`, the names of proofs
/// the answer gives; `default_name`, `PlannerRole::default_proof_name` of the
/// planner.
/// Output: the ids of the proofs (`[6, 15]`), one of `default_name` when no
/// name is usable; an error of the database.
///
/// Called by: [`apply_parameters`].
/// Calls: [`proof_named`]; `objects/evaluation/lab_proof/lab_proof_store.rs`: `create_lab_proof`.
fn chosen_proofs(
    db: &UserDatabase,
    unit: &WorkUnit,
    proposed: &[String],
    default_name: &str,
) -> Result<Vec<i64>, AiError> {
    let mut names = Vec::with_capacity(proposed.len());

    for name in proposed {
        if !name.is_empty() && name.chars().count() <= MAX_PROOF_NAME && !names.contains(name) {
            names.push(name.clone());
        }
    }

    if names.is_empty() {
        names.push(default_name.to_owned());
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

/// The agent the proposal names, among `agents`.
///
/// Input: `agents`, every agent by id; `proposed`, the name the answer
/// gives; `notes`, where a note for a name that is no agent goes.
/// Output: the id of the agent (`Some(3)`); `None` for none or a name that is
/// no agent.
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
///
/// Input: `plan`, the id of a plan to fill; `unit`, the id of one of its work
/// units; `proposal`, from [`read_parameters`].
/// Output: a note for every name left out (`["no document SRS in the plan"]`);
/// the refusals of the plan and of
/// `objects/evaluation/work_unit/work_unit_rules/parameters.rs`:
/// `update_work_unit_parameters`.
///
/// Called by: `objects/evaluation/work_unit/work_unit_ai/filling.rs`: `applied`.
/// Calls: `objects/evaluation/evaluation_plan/evaluation_plan_rules/planning.rs`: `plan_to_fill`;
/// `objects/evaluation/work_unit/work_unit_store.rs`: `read_work_unit`; [`names_of`], [`planner_role`], [`id_of`],
/// [`chosen_agent`], [`chosen_proofs`]; `objects/evaluation/work_unit/work_unit_rules/parameters.rs`:
/// `update_work_unit_parameters`.
pub fn apply_parameters(
    db: &UserDatabase,
    plan: i64,
    unit: i64,
    proposal: &ParametersProposal,
) -> Result<Vec<String>, AiError> {
    let (stored, _) = plan_to_fill(db, plan)?;
    let this = read_work_unit(db, unit)?;
    let names = names_of(db, &stored)?;
    let default_proof = planner_role(db, plan)?.default_proof_name;
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
        input_functional: if proposal.functional {
            chosen_proofs(db, &this, &proposal.proofs, &default_proof)?
        } else {
            Vec::new()
        },
        ai_agent: chosen_agent(&names.agents, proposal.agent.as_ref(), &mut notes),
        guidance: proposal.guidance.clone(),
    };

    update_work_unit_parameters(db, plan, unit, &parameters)?;

    Ok(notes)
}

/// Writes the proposed dependencies of the work unit `unit` of the plan
/// `plan`, its other parameters kept: only the work units of the plan. A
/// dependency that closes a cycle is written; the planner decides, and a
/// note says that the work unit is now a cross reference.
///
/// Input: `plan`, the id of a plan to fill; `unit`, the id of one of its work
/// units; `proposed`, the labels from [`read_dependencies`].
/// Output: a note for every label left out (`["no work unit X in the plan"]`);
/// the refusals of the plan and of
/// `objects/evaluation/work_unit/work_unit_rules/parameters.rs`:
/// `update_work_unit_parameters`.
///
/// Called by: `objects/evaluation/work_unit/work_unit_ai/filling.rs`: `applied`.
/// Calls: `objects/evaluation/evaluation_plan/evaluation_plan_rules/planning.rs`: `plan_to_fill`;
/// `objects/evaluation/work_unit/work_unit_store.rs`: `read_work_units`, `read_work_unit`; [`labels`], [`id_of`];
/// `objects/evaluation/work_unit/work_unit_rules/parameters.rs`: `update_work_unit_parameters`.
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

    use crate::objects::evaluation::evaluation_plan::evaluation_plan_store::read_evaluation_plan;
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
