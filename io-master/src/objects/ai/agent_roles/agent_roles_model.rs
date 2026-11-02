//! The role of an agent (user, 2026-11-02): what the agent is made for, and
//! the parameters of the conversation of that work. Everything that shapes a
//! question and its result is a field of the agent and of its role, so that a
//! laboratory tunes an agent without a change of the program; the program
//! keeps only the form of an answer it reads (the fields of its JSON).
//!
//! A template is a text with placeholders in braces (`{documents}`), each
//! replaced by its part of the question in one pass
//! (`objects/ai/agent/agent_ai.rs`: `filled`); a placeholder the program does
//! not know stays as it is written.
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

    /// The task as the agent is told it: `{task}` (the task of the kind of
    /// the work unit), `{judgement}`, `{form}` (the form of the answer the
    /// program reads).
    pub task_template: String,

    /// The question of a work unit of the documentation: `{work_unit}` (its
    /// JSON), `{label}`, `{standard}`, `{requirement}`, `{guidance}`,
    /// `{dependencies}`, `{documents}`.
    pub documentation_question: String,

    /// The question of a work unit of a functional test: the placeholders of
    /// `documentation_question` and `{proofs}`.
    pub functional_question: String,

    /// Each dependency in the question: `{label}`, `{verdict}`, `{findings}`.
    pub dependency_template: String,

    /// Each document given whole: `{name}`, `{version}` (recorded by the
    /// evaluator), `{text}`.
    pub document_template: String,

    /// Each passage of the documents when they do not fit whole: `{name}`,
    /// `{text}`.
    pub document_passage_template: String,

    /// Each proof of the laboratory: `{name}`, `{date}`, `{tester}`,
    /// `{equipment}`, `{text}`.
    pub proof_template: String,

    /// What the knowledge of the agent is searched with: `{label}`,
    /// `{requirement}`, `{guidance}`.
    pub knowledge_search: String,

    /// What the passages of the documents are searched with: `{label}`,
    /// `{requirement}`, `{guidance}`.
    pub document_search: String,
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

    /// The question of the parameters: `{label}`, `{standard}`,
    /// `{requirement}`, `{parameters}` (their JSON now), `{documents}`,
    /// `{agents}`.
    pub parameters_question: String,

    /// The question of the dependencies: `{label}`, `{requirement}`,
    /// `{dependencies}` (now), `{others}` (a line about each other work unit).
    pub dependencies_question: String,

    /// The line about each other work unit: `{label}`, `{kind}`
    /// ("documentation" or "functional"), `{sentence}` (the first sentence of
    /// its requirement).
    pub summary_template: String,

    /// What the knowledge of the agent is searched with: `{label}`,
    /// `{requirement}`.
    pub knowledge_search: String,

    /// The name of the proof of a functional test whose answer names none.
    pub default_proof_name: String,
}
