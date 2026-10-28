//! A proof of the laboratory: the result of a functional test the
//! laboratory carries out, delivered as text, kept in the table
//! `lab_proofs`. Its place is reserved, with its id, when the plan is
//! made; the text comes with the copy of the plan the evaluator works on.

/// A proof of the laboratory (user, 2026-10-28).
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(dead_code, reason = "a new model (2026-10-28), not used by the code yet")]
pub struct LabProof {
    /// The id of the proof in the database, given when its place is
    /// reserved in the plan.
    pub id: i64,

    /// What the proof is ("results of the scan of the ports"), given by
    /// the planner when the place is reserved.
    pub name: String,

    /// The text of the proof; `None` while the place is only reserved.
    pub proof: Option<String>,
}
