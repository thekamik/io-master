//! An evaluation plan: the work units of an evaluation, with the
//! standards they come from and the documents of the product they need.
//! The planner makes the plan once (for example for routers); every
//! evaluation of a product works on an identical copy of it, in which
//! the evaluator gives the documents and assesses the work units.

use crate::model::work_unit::WorkUnit;

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

    /// The standards of the plan (OSCAL catalogs: the CEM, PP1, PP2):
    /// the reference of the evaluation. The `reference` of every work
    /// unit is one of them.
    pub oscal_documents: Vec<i64>,

    /// The documents of the product (of the client: ST, IXIT, ...),
    /// their places reserved when the plan is made. The
    /// `input_product_documents` of every work unit are among them.
    pub product_documents: Vec<i64>,

    /// The work units, in the order of the standard.
    pub work_units: Vec<WorkUnit>,

    /// `true` when at least one work unit is a `cross_reference`: the
    /// plan cannot be carried out until the planner corrects it.
    pub blocked: bool,
}
