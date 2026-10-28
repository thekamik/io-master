//! A work unit: one piece of the work of an evaluation, as the standard
//! says it (for example a work unit of the CEM), with what it needs
//! before it can be assessed: the work units it depends on, the
//! documents of the manufacturer and the evidence of the laboratory.

/// A work unit of a standard (user, 2026-10-28).
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(dead_code, reason = "a new model (2026-10-28), not used by the code yet")]
pub struct WorkUnit {
    /// The standard the work unit belongs to.
    pub reference: String,
    /// What is to be done in the work unit: the quote of the requirement
    /// from the standard.
    pub requirement: String,
    /// The work units this one depends on: each must be passed before
    /// this one can be assessed.
    pub dependencies: Vec<String>,
    /// The documents needed to find the work unit met.
    pub input_documents: Vec<String>,
    /// The evidence of the laboratory needed to meet the work unit,
    /// delivered as text; `None` when the work unit is conceptual.
    pub input_functional: Option<Vec<String>>,
}
