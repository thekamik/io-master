//! An evaluation plan: the work units of an evaluation, with the
//! standards they come from and the documents of the product they need.
//! The planner makes the plan once (for example for routers); every
//! evaluation of a product works on an identical copy of it, in which
//! the evaluator gives the documents and assesses the work units. A new
//! version of a document (a new ST) is a new evaluation. When its last
//! work unit is assessed, the filled plan is the report.

use crate::model::evaluation::work_unit::{Verdict, WorkUnit};

/// An evaluation plan, or the copy of it for one product (user,
/// 2026-10-28).
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(dead_code, reason = "a new model (2026-10-28), not used by the code yet")]
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

    /// The work units, in the order of the standard.
    pub work_units: Vec<WorkUnit>,

    /// `true` when at least one work unit is a `cross_reference`: the
    /// plan cannot be carried out until the planner corrects it.
    pub blocked: bool,
}

/// Where an evaluation plan is in its life (user, 2026-10-28).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(dead_code, reason = "a new model (2026-10-28), not used by the code yet")]
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
