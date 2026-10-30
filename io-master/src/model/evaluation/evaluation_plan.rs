//! An evaluation plan: the work units of an evaluation, with the
//! standards they come from and the documents of the product they need.
//! The planner makes the plan once (for example for routers); every
//! evaluation of a product works on an identical copy of it, in which
//! the evaluator gives the documents and assesses the work units. A new
//! version of a document (a new ST) is a new evaluation. When its last
//! work unit is assessed, the filled plan is the report.

use crate::model::evaluation::work_unit::Verdict;

/// An evaluation plan, or the copy of it for one product (user,
/// 2026-10-28).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluationPlan {
    /// The id of the plan in the database.
    pub id: i64,

    /// The name of the plan, for the list of plans.
    pub name: String,

    /// What the plan is for.
    pub description: String,

    /// Where the plan is in its life: being planned, or being carried
    /// out in an evaluation.
    pub state: PlanState,

    /// The plan this evaluation was copied from (`EvaluationPlan`);
    /// `None` for a plan. This and the four fields below identify an
    /// evaluation, as ISO/IEC 17025 asks; they are empty in a plan.
    pub source_plan: Option<i64>,

    /// The product evaluated: its name and version ("Router R7, firmware
    /// 2.4.1").
    pub product: String,

    /// The client who ordered the evaluation.
    pub client: String,

    /// The evaluator who carries it out.
    pub evaluator: String,

    /// When the evaluation was started: seconds since 1970 (UTC); `None`
    /// for a plan.
    pub started: Option<i64>,

    /// The standards of the plan (OSCAL catalogs: the CEM, PP1, PP2):
    /// the reference of the evaluation. The `reference` of every work
    /// unit is one of them.
    pub oscal_documents: Vec<i64>,

    /// The documents of the product (of the client: ST, IXIT, ...),
    /// their places reserved when the plan is made. The
    /// `input_product_documents` of every work unit are among them.
    pub product_documents: Vec<i64>,

    /// The verdicts the standards of the plan know: `Pass` and `Fail`
    /// when a plan is made, as Common Criteria; the planner may add
    /// `Inconclusive` (ETSI EN 303 645).
    pub possible_verdicts: Vec<Verdict>,

    /// The work units of the plan (`WorkUnit`), in the order of the
    /// standard. They are rows of the table `work_units` of their own,
    /// read and changed one by one, so that reading a plan does not read
    /// its hundreds of work units and a change of one is a small
    /// transaction (user, 2026-10-28).
    pub work_units: Vec<i64>,

    /// `true` when at least one work unit is a `cross_reference`: the
    /// plan cannot be carried out until the planner corrects it.
    pub blocked: bool,

    /// The agent of the plan (`Agent`; user, 2026-10-30: one field, its
    /// work told by the state). In a plan in planning, the agent of the
    /// planner, which fills the parameters of the work units with its
    /// knowledge ("Fill with AI"); in an evaluation, the agent that
    /// assesses every work unit without an agent of its own. The copy of a
    /// plan does not take it over: the evaluator chooses it.
    pub ai_agent: Option<i64>,
}

/// Where an evaluation plan is in its life (user, 2026-10-28).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanState {
    /// The plan is being made: the planner chooses the standards and
    /// the work units, reserves the places of the documents and the
    /// proofs, and sets the dependencies.
    Plan,
    /// The evaluation is being carried out: the evaluator gives the
    /// documents and the proofs, and the work units are assessed.
    Evaluation,
    /// The last work unit is assessed: the filled plan is the report.
    Report,
}
