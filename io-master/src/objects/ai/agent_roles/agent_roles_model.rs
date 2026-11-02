//! The role of an agent (user, 2026-11-02): what the agent is made for, and
//! the parameters of the conversation of that work. Everything that shapes a
//! question and its result is a field of the agent and of its role, so that a
//! laboratory tunes an agent without a change of the program; the program
//! keeps only the form of an answer it reads (the fields of its JSON).
//!
//! ```text
//! Agent::role ──► Evaluator(EvaluatorRole)   assesses the work units of an evaluation
//!             ──► Planner(PlannerRole)       fills the work units of a plan ("Fill with AI")
//!             ──► Chat                       talks with the user; any agent may talk
//! ```
//!
//! A role is given when the agent is made and never changes: plans and
//! evaluations name the agent for its work.

/// What an agent is made for, with the parameters of that work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentRole {
    /// It assesses the work units of an evaluation.
    Evaluator(EvaluatorRole),
    /// It fills the work units of a plan in planning.
    Planner(PlannerRole),
    /// It only talks with the user.
    Chat,
}

/// The parameters of the assessment of a work unit by an agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluatorRole {
    /// How a work unit is judged, for both kinds of work unit: its scope,
    /// substance before form, the care asked for, what each field of the
    /// answer holds.
    pub judgement: String,

    /// The task of a work unit of the documentation (one without proofs).
    pub documentation_task: String,

    /// The task of a work unit of a functional test (one with proofs).
    pub functional_task: String,

    /// The most passages of the documents given with a question when the
    /// documents do not fit whole into the context of the model.
    pub document_passages: u32,

    /// The most characters of the findings of each dependency given with a
    /// question.
    pub dependency_findings: u32,
}

/// The parameters of the filling of a plan by an agent ("Fill with AI").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannerRole {
    /// The task of the parameters of a work unit, and what each field of the
    /// answer holds.
    pub parameters_task: String,

    /// The task of the dependencies of a work unit, and what its answer holds.
    pub dependencies_task: String,

    /// The most characters of the line about each other work unit in the
    /// question of the dependencies.
    pub summary_characters: u32,
}
