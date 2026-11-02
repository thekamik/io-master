//! The rules of the role of an agent (user, 2026-11-02): the role a new agent
//! is given, with the parameters it starts from, and the parameters as the
//! user changes them, checked. The texts below are only where a new agent
//! starts: from then on they are its own, tuned by the laboratory without a
//! change of the program.
//!
//! ```text
//! new_role("evaluator")  ──► Evaluator: the judgement, the two tasks, 20 passages, 1000 characters
//! new_role("planner")    ──► Planner: the tasks of the parameters and of the dependencies, 200 characters
//! new_role("chat")       ──► Chat
//! checked_role(role)     ──► the texts trimmed and given, the limits in their ranges
//! ```

use crate::objects::ai::agent_roles::agent_roles_model::{AgentRole, EvaluatorRole, PlannerRole};
use crate::objects::ai::agent_roles::agent_roles_store::{
    MAX_DEPENDENCY_FINDINGS, MAX_DOCUMENT_PASSAGES, MAX_SUMMARY_CHARACTERS, MAX_TASK,
};
use crate::objects::error::DomainError;
use crate::objects::text::checked_text;

/// How a new evaluator judges a work unit: its scope, substance before form,
/// care about the substance (user, 2026-11-02), what the extracted text and
/// the recorded version are, and what each field of the answer holds.
const NEW_JUDGEMENT: &str = "Judge only what the requirement of this work unit asks. A fault that belongs to \
another work unit, its dependencies among them, is judged there: tell it in the notes, never in the findings or the \
verdict, unless this requirement itself rests on it.
Fail the work unit when the content its requirement asks for is missing, or is contradicted in a way that touches \
the requirement. An editorial flaw (the format of a date, a broken cross-reference, numbering, layout) goes to the \
notes and does not change the verdict.
Be careful about the substance: when the evidence does not show that the requirement is met, fail the work unit. A \
false fail is corrected by the evaluator, a false pass may never be found.
The text of a document was extracted from its file: its tables may be broken and its formatting (italic, bold, \
underline) is lost. That is no fault of the document; what cannot be decided because of it is told in the notes, for \
the evaluator to check in the original. The version of a document written under its name was recorded by the \
evaluator: it is no part of the document and never a reason for a verdict.
In the answer, \"findings\" holds each thing the requirement asks for and the evidence does not give, one per line, \
or nothing; \"analysis\" how the evidence meets or fails the requirement, briefly; \"vulnerability_notes\" what an \
attacker could use, or nothing; \"notes\" anything else the evaluator should know, or nothing; \"quotes\" the places \
your verdict rests on, word for word: a quote is help for the evaluator, not a condition of the verdict. Decide the \
verdict last, from your findings and analysis. Write briefly and to the point: no repetition of the requirement, no \
general remarks.";

/// The task of a work unit of the documentation for a new evaluator.
const NEW_DOCUMENTATION_TASK: &str = "You assess one work unit of a security evaluation as an experienced evaluator: \
does the product, as its documents describe it, meet the requirement of the work unit? Judge by the substance of the \
documents, as the evaluation methodology means the requirement; evaluations differ, and a requirement met in other \
words or in another place of the document is met.
The documents of the product given in the question are the evidence. The knowledge given above, if any, is help in \
reading the requirement: interpretations of the standard come first, then examples of earlier evaluations and the \
guidance of the laboratory; it is never evidence about this product.";

/// The task of a work unit of a functional test for a new evaluator.
const NEW_FUNCTIONAL_TASK: &str = "You assess one work unit of a security evaluation as an experienced evaluator: do \
the documents of the product and the proofs of the laboratory show that the requirement of the work unit is met? \
Judge by the substance of the evidence, as the evaluation methodology means the requirement; evaluations differ, and \
a requirement met in other words or in another place is met.
The documents of the product and the proofs of the laboratory (the record of each test: who performed it, when and \
with what) given in the question are the evidence. The knowledge given above, if any, is help in reading the \
requirement: interpretations of the standard come first, then examples of earlier evaluations and the guidance of the \
laboratory; it is never evidence about this product. In the quotes, \"document\" may also be the name of a proof \
given.";

/// The passages of the documents and the characters of the findings of a
/// dependency a new evaluator gives with a question.
const NEW_DOCUMENT_PASSAGES: u32 = 20;
const NEW_DEPENDENCY_FINDINGS: u32 = 1000;

/// The task of the parameters of a work unit for a new planner, and what each
/// field of its answer holds.
const NEW_PARAMETERS_TASK: &str = "You plan one work unit of a security evaluation. From its requirement, its \
parameters now and your knowledge, propose its parameters. Use only the documents and agents the question lists.
In the answer, \"documents\" holds the names of the documents of the product it needs; \"functional\" true when it \
needs a functional test of the laboratory, false for a test of the documentation; \"proofs\" the names of the proofs \
of the laboratory its functional test needs; \"agent\" the name of the agent that should assess it, or null; \
\"guidance\" the guidance for the evaluator on this work unit, as the methodology gives it, improved where your \
knowledge helps: only the guidance itself, never remarks on this question, on what was given or on what is missing; \
the guidance given unchanged when you have nothing to add.";

/// The task of the dependencies of a work unit for a new planner.
const NEW_DEPENDENCIES_TASK: &str = "You plan the order of the work of a security evaluation. From the requirement of \
one work unit, the other work units of the plan and your knowledge, name the work units whose results this one needs, \
so that they are done before it. Use only the labels the question lists; an empty list when it needs none.";

/// The characters of the line about each other work unit a new planner gives
/// in the question of the dependencies.
const NEW_SUMMARY_CHARACTERS: u32 = 200;

/// The role of a new agent, with the parameters it starts from.
///
/// Input: `name`, the role the page sends: `"evaluator"`, `"planner"` or
/// `"chat"`.
/// Output: the role (`AgentRole::Evaluator(EvaluatorRole { document_passages:
/// 20, .. })`); `DomainError::Refused` for another name.
///
/// Called by: `objects/ai/agent/agent_rules.rs`: `new_agent`.
pub fn new_role(name: &str) -> Result<AgentRole, DomainError> {
    match name {
        "evaluator" => Ok(AgentRole::Evaluator(EvaluatorRole {
            judgement: NEW_JUDGEMENT.to_owned(),
            documentation_task: NEW_DOCUMENTATION_TASK.to_owned(),
            functional_task: NEW_FUNCTIONAL_TASK.to_owned(),
            document_passages: NEW_DOCUMENT_PASSAGES,
            dependency_findings: NEW_DEPENDENCY_FINDINGS,
        })),
        "planner" => Ok(AgentRole::Planner(PlannerRole {
            parameters_task: NEW_PARAMETERS_TASK.to_owned(),
            dependencies_task: NEW_DEPENDENCIES_TASK.to_owned(),
            summary_characters: NEW_SUMMARY_CHARACTERS,
        })),
        "chat" => Ok(AgentRole::Chat),
        _ => Err(DomainError::Refused(format!("no role \"{name}\" of an agent"))),
    }
}

/// The parameters of an evaluator as the user changed them, checked: each
/// text trimmed and given, each limit in its range.
///
/// Input: `role`, the parameters as typed.
/// Output: the parameters to keep; `DomainError::Refused` for an empty text,
/// one over `MAX_TASK`, or a limit out of its range.
///
/// Called by: [`checked_role`].
/// Calls: `objects/text.rs`: `checked_text`.
fn checked_evaluator(role: &EvaluatorRole) -> Result<EvaluatorRole, DomainError> {
    ensure!(
        (1..=MAX_DOCUMENT_PASSAGES).contains(&role.document_passages),
        DomainError::Refused(format!("the passages of the documents: from 1 to {MAX_DOCUMENT_PASSAGES}"))
    );
    ensure!(
        role.dependency_findings <= MAX_DEPENDENCY_FINDINGS,
        DomainError::Refused(format!("the findings of a dependency: at most {MAX_DEPENDENCY_FINDINGS} characters"))
    );

    Ok(EvaluatorRole {
        judgement: checked_text(&role.judgement, "the judgement", MAX_TASK)?,
        documentation_task: checked_text(&role.documentation_task, "the task of the documentation", MAX_TASK)?,
        functional_task: checked_text(&role.functional_task, "the task of a functional test", MAX_TASK)?,
        document_passages: role.document_passages,
        dependency_findings: role.dependency_findings,
    })
}

/// The parameters of a planner as the user changed them, checked: each text
/// trimmed and given, the limit in its range.
///
/// Input: `role`, the parameters as typed.
/// Output: the parameters to keep; `DomainError::Refused` for an empty text,
/// one over `MAX_TASK`, or a limit out of its range.
///
/// Called by: [`checked_role`].
/// Calls: `objects/text.rs`: `checked_text`.
fn checked_planner(role: &PlannerRole) -> Result<PlannerRole, DomainError> {
    ensure!(
        (1..=MAX_SUMMARY_CHARACTERS).contains(&role.summary_characters),
        DomainError::Refused(format!("the line about a work unit: from 1 to {MAX_SUMMARY_CHARACTERS} characters"))
    );

    Ok(PlannerRole {
        parameters_task: checked_text(&role.parameters_task, "the task of the parameters", MAX_TASK)?,
        dependencies_task: checked_text(&role.dependencies_task, "the task of the dependencies", MAX_TASK)?,
        summary_characters: role.summary_characters,
    })
}

/// The role of an agent as the user changed its parameters, checked.
///
/// Input: `role`, the role as typed.
/// Output: the role to keep; the refusals of [`checked_evaluator`] and
/// [`checked_planner`].
///
/// Called by: `objects/ai/agent/agent_rules.rs`: `checked`.
/// Calls: [`checked_evaluator`], [`checked_planner`].
pub fn checked_role(role: &AgentRole) -> Result<AgentRole, DomainError> {
    match role {
        AgentRole::Evaluator(evaluator) => Ok(AgentRole::Evaluator(checked_evaluator(evaluator)?)),
        AgentRole::Planner(planner) => Ok(AgentRole::Planner(checked_planner(planner)?)),
        AgentRole::Chat => Ok(AgentRole::Chat),
    }
}

/// The parameters of the evaluator `role`, refused for an agent of another
/// role: a work unit and an evaluation are assessed only by an evaluator.
///
/// Input: `role`, the role of an agent; `name`, its name, for the refusal.
/// Output: its parameters; `DomainError::Refused` ("Planner is no evaluator").
///
/// Called by: `objects/evaluation/work_unit/work_unit_ai/assessment_question.rs`: `question_of`;
/// `objects/evaluation/evaluation_plan/evaluation_plan_rules/evaluation.rs`: `new_evaluation`,
/// `change_agent`; `objects/evaluation/work_unit/work_unit_rules/parameters.rs`:
/// `update_work_unit_parameters`.
pub fn evaluator_of<'a>(role: &'a AgentRole, name: &str) -> Result<&'a EvaluatorRole, DomainError> {
    match role {
        AgentRole::Evaluator(evaluator) => Ok(evaluator),
        _ => Err(DomainError::Refused(format!("the agent {name} is no evaluator"))),
    }
}

/// The parameters of the planner `role`, refused for an agent of another
/// role: a plan is filled only by a planner.
///
/// Input: `role`, the role of an agent; `name`, its name, for the refusal.
/// Output: its parameters; `DomainError::Refused` ("Evaluator is no planner").
///
/// Called by: `objects/evaluation/evaluation_plan/evaluation_plan_rules/planning.rs`: `update_plan`,
/// `plan_to_fill`; `objects/evaluation/work_unit/work_unit_ai/proposal.rs`: `planner_role`.
pub fn planner_of<'a>(role: &'a AgentRole, name: &str) -> Result<&'a PlannerRole, DomainError> {
    match role {
        AgentRole::Planner(planner) => Ok(planner),
        _ => Err(DomainError::Refused(format!("the agent {name} is no planner"))),
    }
}

#[cfg(test)]
mod tests {
    //! Tests of the roles: a new agent starts from the parameters of its role,
    //! the parameters as the user changes them are checked, a role is asked
    //! for where its work is.

    use super::*;

    #[test]
    fn a_new_role_starts_from_its_parameters_and_an_unknown_one_is_refused() {
        let AgentRole::Evaluator(evaluator) = new_role("evaluator").unwrap() else {
            panic!("an evaluator was asked for");
        };

        assert_eq!((evaluator.document_passages, evaluator.dependency_findings), (20, 1000));
        assert!(evaluator.judgement.contains("Judge only what") && evaluator.functional_task.contains("proofs"));
        assert!(matches!(
            new_role("planner").unwrap(),
            AgentRole::Planner(PlannerRole { summary_characters: 200, .. })
        ));
        assert_eq!(new_role("chat").unwrap(), AgentRole::Chat);
        assert!(matches!(new_role("auditor"), Err(DomainError::Refused(_))));
    }

    #[test]
    fn the_parameters_of_a_role_are_trimmed_and_kept_in_their_ranges() {
        let AgentRole::Evaluator(evaluator) = new_role("evaluator").unwrap() else {
            panic!("an evaluator was asked for");
        };
        let changed = EvaluatorRole { judgement: "  Be strict.  ".to_owned(), ..evaluator.clone() };

        assert!(matches!(
            checked_role(&AgentRole::Evaluator(changed)).unwrap(),
            AgentRole::Evaluator(EvaluatorRole { ref judgement, .. }) if judgement == "Be strict."
        ));

        for wrong in [
            EvaluatorRole { documentation_task: " ".to_owned(), ..evaluator.clone() },
            EvaluatorRole { document_passages: 0, ..evaluator.clone() },
            EvaluatorRole { dependency_findings: MAX_DEPENDENCY_FINDINGS + 1, ..evaluator.clone() },
        ] {
            assert!(matches!(checked_role(&AgentRole::Evaluator(wrong)), Err(DomainError::Refused(_))));
        }

        assert!(evaluator_of(&new_role("planner").unwrap(), "Planner").is_err());
        assert!(planner_of(&new_role("evaluator").unwrap(), "Evaluator").is_err());
    }
}
