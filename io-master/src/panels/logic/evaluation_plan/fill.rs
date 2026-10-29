//! "Fill with AI" (user, 2026-10-29): the agent of the planner of a plan
//! goes through every work unit of the plan, in its order, and proposes
//! all its parameters; they are written at once, over what was there, as
//! if the planner had written them.
//!
//! ```text
//! fill_units ──► the work units of the plan (in planning, with an agent of the planner)
//! for each:  fill_question ──► Question: the instructions of the agent, the task, its knowledge;
//!                               the work unit, its parameters now, the labels of the plan,
//!                               the documents, the agents
//!            (ai/ask.rs: prepare, send, record into the journal of the agent)
//!            read_proposal ──► Proposal (the JSON of the answer)
//!            apply_proposal ──► only what the plan holds; a dependency that would close a cycle
//!                               is left out ──► update_work_unit_parameters; the notes of what was left out
//! ```
//!
//! A dependency on a work unit not filled yet is allowed: the cycle is
//! checked on the dependencies of the plan as they are when the answer is
//! written, so no cross reference is ever made by the filling.

use std::collections::HashMap;

use serde_json::{Value, json};

use crate::ai::ask::Question;
use crate::db::ai::agent::{read_agent, read_agents};
use crate::db::database::Database;
use crate::db::evaluation::evaluation_plan::read_evaluation_plan;
use crate::db::evaluation::lab_proof::{create_lab_proof, read_lab_proof};
use crate::db::evaluation::oscal_standard_document::read_oscal_standard_document;
use crate::db::evaluation::product_document::read_product_document;
use crate::db::evaluation::work_unit::{MAX_DEPENDENCIES, MAX_DOCUMENTS, MAX_PROOFS, read_work_unit, read_work_units};
use crate::model::ai::chat::{Message, Speaker};
use crate::model::evaluation::evaluation_plan::EvaluationPlan;
use crate::model::evaluation::lab_proof::LabProof;
use crate::model::evaluation::work_unit::WorkUnit;
use crate::panels::logic::agents::agent::agent_knowledge;
use crate::panels::logic::evaluation_plan::error::PlanError;
use crate::panels::logic::evaluation_plan::plans::check_planning;
use crate::panels::logic::evaluation_plan::work_units::{WorkUnitParameters, update_work_unit_parameters};

/// The longest guidance taken from an answer, in characters.
const MAX_GUIDANCE: usize = 20_000;

/// The longest name of a proof taken from an answer, in characters.
const MAX_PROOF_NAME: usize = 200;

/// The name of the proof of a functional test the answer names none for.
const DEFAULT_PROOF: &str = "Results of the functional test";

/// The task of the agent of the planner, given after its own instructions.
const TASK: &str = "You plan one work unit of a security evaluation. From its requirement, its parameters now, the \
other work units of the plan and your knowledge, propose all its parameters. Answer with one JSON object and nothing \
else:\n\
{\"dependencies\": [the labels of the work units of the plan that must be done before this one],\n \
\"documents\": [the names of the documents of the product it needs],\n \
\"functional\": true when it needs a functional test of the laboratory, false for a test of the documentation,\n \
\"proofs\": [the names of the proofs of the laboratory its functional test needs],\n \
\"agent\": the name of the agent that should assess it, or null,\n \
\"guidance\": the guidance for the evaluator, whole}\n\
Use only the labels, documents and agents the question lists. A dependency must never lead back to this work unit.";

/// The parameters an answer proposes, as names, before they are checked
/// against the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposal {
    pub dependencies: Vec<String>,
    pub documents: Vec<String>,
    pub functional: bool,
    pub proofs: Vec<String>,
    pub agent: Option<String>,
    pub guidance: String,
}

/// The plan `plan`, refused unless it is in planning and names an agent of
/// the planner; gives the plan and its agent.
///
/// Called by: [`fill_units`], [`fill_question`].
/// Calls: `read_evaluation_plan`, `plans::check_planning`.
fn plan_to_fill(db: &Database, plan: i64) -> Result<(EvaluationPlan, i64), PlanError> {
    let stored = read_evaluation_plan(db, plan)?;

    check_planning(&stored)?;

    let Some(agent) = stored.planning_agent else {
        return Err(PlanError::Refused("the plan has no agent of the planner: choose one in its data".to_owned()));
    };

    Ok((stored, agent))
}

/// The work units of the plan `plan` to fill, in its order; refused unless
/// the plan is in planning and names an agent of the planner.
///
/// Called by: `panels/views/evaluation_plan/fill.rs` (the start of a filling).
/// Calls: [`plan_to_fill`].
pub fn fill_units(db: &Database, plan: i64) -> Result<Vec<i64>, PlanError> {
    let (stored, _) = plan_to_fill(db, plan)?;

    ensure!(!stored.work_units.is_empty(), PlanError::Refused("the plan has no work unit to fill".to_owned()));

    Ok(stored.work_units)
}

/// The label of each work unit of `units`, by its id.
///
/// Called by: [`fill_question`], [`apply_proposal`].
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
/// Called by: [`current_parameters`].
fn named(ids: &[i64], names: &HashMap<i64, String>) -> Vec<String> {
    let mut found = Vec::with_capacity(ids.len());

    for id in ids {
        if let Some(name) = names.get(id) {
            found.push(name.clone());
        }
    }

    found
}

/// The parameters of the work unit `unit` now, as the question shows them
/// (names, not ids).
///
/// Called by: [`fill_question`].
/// Calls: [`named`], `read_lab_proof`.
fn current_parameters(
    db: &Database,
    unit: &WorkUnit,
    labels: &HashMap<i64, String>,
    documents: &HashMap<i64, String>,
    agents: &HashMap<i64, String>,
) -> Result<Value, PlanError> {
    let mut proofs = Vec::new();

    for id in unit.input_functional.as_deref().unwrap_or_default() {
        proofs.push(read_lab_proof(db, *id)?.name);
    }

    let agent = match unit.ai_agent {
        Some(id) => agents.get(&id).cloned(),
        None => None,
    };

    Ok(json!({
        "dependencies": named(&unit.dependencies, labels),
        "documents": named(&unit.input_product_documents, documents),
        "functional": unit.input_functional.is_some(),
        "proofs": proofs,
        "agent": agent,
        "guidance": unit.guidance,
    }))
}

/// The names a work unit of a plan may name, by their ids.
struct Names {
    /// The documents of the product of the plan.
    documents: HashMap<i64, String>,
    /// Every agent of the user.
    agents: HashMap<i64, String>,
}

/// The names of the documents of the plan and of every agent, by their
/// ids.
///
/// Called by: [`fill_question`], [`apply_proposal`].
/// Calls: `read_product_document`, `read_agents`.
fn names_of(db: &Database, plan: &EvaluationPlan) -> Result<Names, PlanError> {
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
/// work unit `unit`: the instructions of the agent and the task, with the
/// knowledge of the agent that fits the work unit; the work unit, its
/// parameters now and what the plan lets it name.
///
/// Called by: `panels/views/evaluation_plan/fill.rs` (each work unit).
/// Calls: [`plan_to_fill`], `read_agent`, `agent_knowledge`, [`names_of`],
/// [`current_parameters`].
pub fn fill_question(db: &Database, plan: i64, unit: i64) -> Result<Question, PlanError> {
    let (stored, agent_id) = plan_to_fill(db, plan)?;

    ensure!(stored.work_units.contains(&unit), PlanError::Refused("the work unit is not one of the plan".to_owned()));

    let agent = read_agent(db, agent_id)?;
    let units = read_work_units(db, plan)?;
    let this = read_work_unit(db, unit)?;
    let labels = labels(&units);
    let Names { documents, agents } = names_of(db, &stored)?;
    let knowledge = agent_knowledge(db, agent_id, &format!("{} {}", this.label, this.requirement))?;
    let mut agent_names: Vec<String> = agents.values().cloned().collect();
    let mut system = format!("{}\n\n{TASK}", agent.instructions);

    agent_names.sort();

    if !knowledge.is_empty() {
        system.push_str("\n\nYour knowledge for this work unit:\n\n");
        system.push_str(&knowledge.join("\n\n---\n\n"));
    }

    let text = format!(
        "Work unit: {}\nStandard: {}\n\nRequirement:\n{}\n\nIts parameters now:\n{}\n\nThe work units of the plan: \
         {}\nThe documents of the product: {}\nThe agents: {}",
        this.label,
        read_oscal_standard_document(db, this.reference)?.title,
        this.requirement,
        current_parameters(db, &this, &labels, &documents, &agents)?,
        named(&stored.work_units, &labels).join(", "),
        named(&stored.product_documents, &documents).join(", "),
        agent_names.join(", "),
    );

    Ok(Question {
        integration: agent.integration,
        agent: Some(agent_id),
        work_unit: Some(unit),
        system,
        messages: vec![Message { speaker: Speaker::User, text, time: 0 }],
    })
}

/// The strings of the list `field` of the answer, at most `max`; what is no
/// string is left out.
///
/// Called by: [`read_proposal`].
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

/// The proposal of an answer: the JSON object in its text (a fence of
/// Markdown or words around it are passed over). Refused when the answer
/// holds no JSON object.
///
/// Called by: `panels/views/evaluation_plan/fill.rs` (each answer).
/// Calls: [`strings`].
pub fn read_proposal(answer: &str) -> Result<Proposal, PlanError> {
    let (Some(start), Some(end)) = (answer.find('{'), answer.rfind('}')) else {
        return Err(PlanError::Refused("the answer holds no JSON object".to_owned()));
    };
    let object = answer.get(start..=end).unwrap_or_default();
    let value: Value = serde_json::from_str(object)
        .map_err(|error| PlanError::Refused(format!("the answer is no JSON object: {error}")))?;
    let guidance = value.get("guidance").and_then(Value::as_str).unwrap_or_default().trim();

    ensure!(
        guidance.chars().count() <= MAX_GUIDANCE,
        PlanError::Refused(format!("a guidance over {MAX_GUIDANCE} characters"))
    );

    Ok(Proposal {
        dependencies: strings(&value, "dependencies", MAX_DEPENDENCIES),
        documents: strings(&value, "documents", MAX_DOCUMENTS),
        functional: value.get("functional").and_then(Value::as_bool).unwrap_or(false),
        proofs: strings(&value, "proofs", MAX_PROOFS),
        agent: value.get("agent").and_then(Value::as_str).map(str::trim).map(str::to_owned),
        guidance: guidance.to_owned(),
    })
}

/// The id of the name `name` among `names`, its case and the spaces around
/// it passed over.
///
/// Called by: [`apply_proposal`], [`chosen_dependencies`].
fn id_of(names: &HashMap<i64, String>, name: &str) -> Option<i64> {
    let wanted = name.trim().to_lowercase();

    for (id, known) in names {
        if known.trim().to_lowercase() == wanted {
            return Some(*id);
        }
    }

    None
}

/// Whether the work unit `from` leads to the work unit `to` through the
/// dependencies `graph`: a dependency of `to` on `from` would then close a
/// cycle. An explicit stack (rule 1 of CLAUDE.md); each work unit visited
/// once.
///
/// Called by: [`chosen_dependencies`].
fn leads_to(graph: &HashMap<i64, Vec<i64>>, from: i64, to: i64) -> bool {
    let mut stack = vec![from];
    let mut seen: Vec<i64> = Vec::with_capacity(graph.len());

    for _ in 0..=graph.len() * graph.len().max(1) {
        let Some(current) = stack.pop() else { return false };

        if current == to {
            return true;
        }

        if seen.contains(&current) {
            continue;
        }

        seen.push(current);
        stack.extend(graph.get(&current).cloned().unwrap_or_default());
    }

    false
}

/// The dependencies of the work unit `unit` the proposal names that the
/// plan holds and that close no cycle, with a note for each one left out.
///
/// Called by: [`apply_proposal`].
/// Calls: [`id_of`], [`leads_to`].
fn chosen_dependencies(units: &[WorkUnit], unit: i64, proposed: &[String], notes: &mut Vec<String>) -> Vec<i64> {
    let names = labels(units);
    let mut graph: HashMap<i64, Vec<i64>> = HashMap::with_capacity(units.len());
    let mut chosen = Vec::with_capacity(proposed.len());

    for other in units {
        graph.insert(other.id, if other.id == unit { Vec::new() } else { other.dependencies.clone() });
    }

    for label in proposed {
        match id_of(&names, label) {
            None => notes.push(format!("no work unit {label} in the plan")),
            Some(id) if id == unit => notes.push("a dependency on itself left out".to_owned()),
            Some(id) if chosen.contains(&id) => {}
            Some(id) if leads_to(&graph, id, unit) => notes.push(format!("{label} would close a cycle; left out")),
            Some(id) => {
                chosen.push(id);
                graph.insert(unit, chosen.clone());
            }
        }
    }

    chosen
}

/// The proof of the work unit `unit` named `name` (its case passed over),
/// if it has one.
///
/// Called by: [`chosen_proofs`].
/// Calls: `read_lab_proof`.
fn proof_named(db: &Database, unit: &WorkUnit, name: &str) -> Result<Option<i64>, PlanError> {
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
/// Called by: [`apply_proposal`].
/// Calls: [`proof_named`], `create_lab_proof`.
fn chosen_proofs(db: &Database, unit: &WorkUnit, proposed: &[String]) -> Result<Vec<i64>, PlanError> {
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

/// Writes the proposal for the work unit `unit` of the plan `plan`: only
/// the work units, documents and agents the plan and the user hold; a
/// dependency that would close a cycle is left out. Gives a note for every
/// name left out.
///
/// Called by: `panels/views/evaluation_plan/fill.rs` (each answer).
/// Calls: [`plan_to_fill`], [`chosen_dependencies`], [`names_of`],
/// [`id_of`], [`chosen_proofs`], `update_work_unit_parameters`.
pub fn apply_proposal(db: &Database, plan: i64, unit: i64, proposal: &Proposal) -> Result<Vec<String>, PlanError> {
    let (stored, _) = plan_to_fill(db, plan)?;
    let units = read_work_units(db, plan)?;
    let this = read_work_unit(db, unit)?;
    let mut notes = Vec::new();
    let dependencies = chosen_dependencies(&units, unit, &proposal.dependencies, &mut notes);
    let Names { documents: document_names, agents: agent_names } = names_of(db, &stored)?;
    let mut documents = Vec::with_capacity(proposal.documents.len());

    for name in &proposal.documents {
        match id_of(&document_names, name) {
            Some(id) if !documents.contains(&id) => documents.push(id),
            Some(_) => {}
            None => notes.push(format!("no document {name} in the plan")),
        }
    }

    let agent = match &proposal.agent {
        Some(name) if !name.is_empty() => {
            let found = id_of(&agent_names, name);

            if found.is_none() {
                notes.push(format!("no agent {name}"));
            }

            found
        }
        _ => None,
    };
    let proofs = if proposal.functional { chosen_proofs(db, &this, &proposal.proofs)? } else { Vec::new() };
    let parameters = WorkUnitParameters {
        dependencies,
        input_product_documents: documents,
        input_functional: proofs,
        ai_agent: agent,
        guidance: proposal.guidance.clone(),
    };

    update_work_unit_parameters(db, plan, unit, &parameters)?;

    Ok(notes)
}

#[cfg(test)]
mod tests {
    //! Tests of the filling: the question, the reading of an answer, the
    //! writing of a proposal without a cycle.

    use super::*;

    use crate::db::ai::agent::create_agent;
    use crate::db::ai::integration::create_integration;
    use crate::db::evaluation::evaluation_plan::update_evaluation_plan;
    use crate::panels::logic::evaluation_plan::plans::{add_document_place, create_plan, update_plan};
    use crate::panels::logic::evaluation_plan::standards::import_standard;
    use crate::panels::logic::evaluation_plan::work_units::add_all_work_units;
    use crate::test_support::database::{agent, database, integration};

    const NIST_SAMPLE: &str = include_str!("../../../../oscal/examples/basic-catalog.json");

    /// A plan of the sample of NIST with its four work units, a document
    /// "ST" and an agent of the planner; gives the plan and its work units.
    fn plan_to_test(db: &Database) -> (i64, Vec<i64>) {
        let standard = import_standard(db, NIST_SAMPLE).unwrap();
        let plan = create_plan(db, "Sample", "").unwrap();
        let stored = read_evaluation_plan(db, plan).unwrap();

        update_plan(db, &EvaluationPlan { oscal_documents: vec![standard], ..stored }).unwrap();
        add_document_place(db, plan, "ST").unwrap();
        add_all_work_units(db, plan, standard).unwrap();

        let integration = create_integration(db, &integration()).unwrap();
        let planner = create_agent(db, &agent(integration)).unwrap();
        let stored = read_evaluation_plan(db, plan).unwrap();

        update_evaluation_plan(db, &EvaluationPlan { planning_agent: Some(planner), ..stored.clone() }).unwrap();

        (plan, stored.work_units)
    }

    #[test]
    fn a_plan_without_an_agent_of_the_planner_is_not_filled() {
        let (_directory, db) = database();
        let (plan, _) = plan_to_test(&db);
        let stored = read_evaluation_plan(&db, plan).unwrap();

        assert_eq!(fill_units(&db, plan).unwrap().len(), 4);

        update_evaluation_plan(&db, &EvaluationPlan { planning_agent: None, ..stored }).unwrap();

        assert!(matches!(fill_units(&db, plan), Err(PlanError::Refused(_))));
    }

    #[test]
    fn the_question_shows_the_work_unit_and_what_it_may_name() {
        let (_directory, db) = database();
        let (plan, units) = plan_to_test(&db);
        let question = fill_question(&db, plan, units[0]).unwrap();
        let text = &question.messages[0].text;

        assert_eq!((question.agent.is_some(), question.work_unit), (true, Some(units[0])));
        assert!(question.system.contains("You assess work units of the CEM.") && question.system.contains("JSON"));
        assert!(text.contains("Work unit: 1.1.1") && text.contains("1.1.2, 2.1.1, 2.1.2"));
        assert!(text.contains("The documents of the product: ST") && text.contains("The agents: CC evaluator"));
    }

    #[test]
    fn an_answer_is_read_from_its_json_object() {
        let answer = "Here it is:\n```json\n{\"dependencies\": [\"1.1.1\", 7], \"documents\": [\"ST\"], \
                      \"functional\": true, \"proofs\": [\"Scan\"], \"agent\": null, \"guidance\": \" Read. \"}\n```";
        let proposal = read_proposal(answer).unwrap();

        assert_eq!(proposal.dependencies, vec!["1.1.1"]);
        assert_eq!((proposal.functional, proposal.agent, proposal.guidance.as_str()), (true, None, "Read."));
        assert!(matches!(read_proposal("I cannot."), Err(PlanError::Refused(_))));
    }

    #[test]
    fn a_proposal_is_written_without_a_cycle_and_what_the_plan_lacks_is_noted() {
        let (_directory, db) = database();
        let (plan, units) = plan_to_test(&db);
        let proposal = |dependencies: &[&str]| Proposal {
            dependencies: dependencies.iter().map(|label| (*label).to_owned()).collect(),
            documents: vec!["st".to_owned(), "IXIT".to_owned()],
            functional: true,
            proofs: vec!["Scan of the ports".to_owned()],
            agent: Some("CC evaluator".to_owned()),
            guidance: "Check the roles.".to_owned(),
        };

        // 1.1.1 depends on 2.1.2, which is not filled yet: allowed.
        assert_eq!(
            apply_proposal(&db, plan, units[0], &proposal(&["2.1.2"])).unwrap(),
            vec!["no document IXIT in the plan"]
        );

        // 2.1.2 on 1.1.1 would close a cycle: left out; 1.1.2 is taken.
        let notes = apply_proposal(&db, plan, units[3], &proposal(&["1.1.1", "1.1.2", "9.9"])).unwrap();
        let filled = read_work_unit(&db, units[3]).unwrap();

        assert!(notes.iter().any(|note| note.contains("1.1.1 would close a cycle")));
        assert!(notes.iter().any(|note| note.contains("no work unit 9.9")));
        assert_eq!(filled.dependencies, vec![units[1]]);
        assert_eq!(filled.guidance, "Check the roles.");
        assert!(filled.ai_agent.is_some() && filled.input_functional.as_ref().map(Vec::len) == Some(1));
        assert!(!read_evaluation_plan(&db, plan).unwrap().blocked);

        // Filled again, the proof of the same name is kept.
        let proof = filled.input_functional.clone();

        apply_proposal(&db, plan, units[3], &proposal(&["1.1.2"])).unwrap();
        assert_eq!(read_work_unit(&db, units[3]).unwrap().input_functional, proof);
    }
}
