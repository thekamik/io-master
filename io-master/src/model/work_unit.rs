//! A work unit: one piece of the work of an evaluation, as the standard
//! says it (for example a work unit of the CEM), with what it needs
//! before it can be assessed (the work units it depends on, the
//! documents of the manufacturer, the evidence of the laboratory) and
//! the results of its assessment. Every other object is named by its id
//! in the database.

/// A work unit of a standard (user, 2026-10-28).
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(dead_code, reason = "a new model (2026-10-28), not used by the code yet")]
pub struct WorkUnit {
    /// The id of the work unit in the database.
    pub id: i64,

    /// The standard the work unit belongs to.
    pub reference: i64,

    /// The identifier of the work unit in its standard ("ADV_ARC.1-1"),
    /// as the standard gives it: the only name of the work unit
    pub label: String,

    /// Where the work unit is in the OSCAL catalog of `reference`: the
    /// `id` of its control in the JSON ("APE_INT.1-1" in the CEM,
    /// "s1.1.1" where the label is "1.1.1").
    pub oscal_key: String,

    /// What is to be done in the work unit: the quote of the requirement
    /// from the standard.
    pub requirement: String,

    /// The explanations and application notes of the standard for the
    /// requirement, given to the AI with it.
    pub guidance: String,

    /// The work units this one depends on: each must be passed before
    /// this one can be assessed.
    pub dependencies: Vec<i64>,

    /// `true` when the dependencies of the work unit close a cycle (a
    /// cross reference: the work unit depends, directly or through
    /// others, on itself), a mistake of the planner; `false` when a work
    /// unit is made. A plan with one such work unit is blocked.
    pub cross_reference: bool,

    /// The documents of the product (of the client: ST, IXIT, ...)
    /// needed to find the work unit met; never a standard, which is
    /// `reference`.
    pub input_product_documents: Vec<i64>,

    /// The proofs of the laboratory (`LabProof`) needed to meet the work
    /// unit, delivered as text. `None` makes the work unit one of the
    /// documentation: no results of functional tests are needed (user,
    /// 2026-10-28: no separate field of the kind).
    pub input_functional: Option<Vec<i64>>,

    /// The AI agent made in the program that works on the work unit;
    /// `None` when none is assigned.
    pub ai_agent: Option<i64>,

    /// The results of the assessment of the work unit.
    pub findings: String,
    pub analysis: String,
    pub vulnerability_notes: String,
    pub notes: String,

    /// The verdict of the assessment; `None` while the work unit is not
    /// assessed. A work unit that depends on this one may be assessed
    /// only when it is `Met`.
    pub verdict: Option<Verdict>,
}

/// The verdict of the assessment of a work unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(dead_code, reason = "a new model (2026-10-28), not used by the code yet")]
pub enum Verdict {
    Met,
    NotMet,
    Inconclusive,
}
