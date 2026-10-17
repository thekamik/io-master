//! The types of an evaluation project (step 29 of `docs/todo.md`):
//! a test project imported for one product, with the results of its
//! requirements and its journal.
//!
//! ```text
//! test project (evaluation/)  ──import──►  evaluation project
//!                                           fields     name, product, its version, manufacturer, evaluator
//!                                           origin     the test project it was made from (as text)
//!                                           structure  schemes, tiles, links: a frozen copy
//!                                           results    four text fields of every requirement
//!                                           proposals  texts of the AI, waiting for the evaluator
//!                                           journal    every step, append-only
//! ```
//!
//! **The structure is frozen.** The schemes, tiles and links are a copy
//! of the accepted part of the test project; later changes of the test
//! project do not touch the evaluation, and the evaluation never
//! changes them. They use the types of `model/test_projects.rs`; in a
//! copy, `Scheme::project` is the number of the evaluation and
//! `Tile::scheme` the number of a scheme of the evaluation.
//!
//! **No text of the AI becomes a result on its own** (user,
//! 2026-10-17). The AI writes a [`Proposal`]; it becomes the
//! [`RequirementResult`] of its field only when the evaluator accepts
//! it, as it is or corrected. The [`ResultOrigin`] keeps how the
//! result came about, and the journal keeps every text.
//!
//! **No fixed verdicts** (user, 2026-10-17): the program supports many
//! standards, so the verdict is text in the field *Assessment and
//! verdict proposition*, written according to the standard.

use crate::model::test_projects::{Link, Scheme, Tile};
use crate::model::text::{Invalid, check_text};

/// The longest name of an evaluation, in characters.
pub const MAX_NAME: usize = 300;

/// The longest product, version, manufacturer or evaluator.
pub const MAX_SHORT: usize = 200;

/// The longest text of a result or a proposal, in characters: an
/// analysis may run over many pages.
pub const MAX_RESULT: usize = 200_000;

/// What the person gives when a test project is imported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluationFields {
    /// The name of the evaluation, unique on this installation.
    pub name: String,
    /// The product under evaluation (the TOE).
    pub product: String,
    pub product_version: String,
    pub manufacturer: String,
    /// Who carries out the evaluation.
    pub evaluator: String,
}

/// The test project an evaluation was made from, kept as text: the
/// test project may be changed or deleted later.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Origin {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
}

/// A stored evaluation, without its structure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evaluation {
    pub id: i64,
    pub fields: EvaluationFields,
    pub origin: Origin,
    /// When it was imported, in seconds since 1970 (UTC).
    pub imported: i64,
}

/// An evaluation with its frozen structure.
#[derive(Debug, Clone, PartialEq)]
pub struct EvaluationData {
    pub evaluation: Evaluation,
    /// The documents of the standard it uses (shared with the test
    /// projects: the quotes and the viewer need them).
    pub documents: Vec<i64>,
    /// In the order of the list of the test project.
    pub schemes: Vec<Scheme>,
    pub tiles: Vec<Tile>,
    pub links: Vec<Link>,
}

/// What an import left out: the parts of the test project that were
/// still proposals of the AI, never accepted by a person.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Skipped {
    /// Proposed schemes (with all their tiles).
    pub schemes: usize,
    /// Proposed tiles, and the tiles of proposed schemes.
    pub tiles: usize,
    /// Links that touch a tile left out.
    pub links: usize,
}

/// The four results of every requirement (user, 2026-10-17).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResultField {
    Findings,
    Analysis,
    /// Assessment and verdict proposition.
    Assessment,
    /// Vulnerabilities and notes.
    Vulnerabilities,
}

impl ResultField {
    /// Every field, in the order of the report.
    pub const ALL: [ResultField; 4] =
        [ResultField::Findings, ResultField::Analysis, ResultField::Assessment, ResultField::Vulnerabilities];

    /// The name stored in the database.
    pub fn name(self) -> &'static str {
        match self {
            ResultField::Findings => "findings",
            ResultField::Analysis => "analysis",
            ResultField::Assessment => "assessment",
            ResultField::Vulnerabilities => "vulnerabilities",
        }
    }

    pub fn from_name(name: &str) -> Option<ResultField> {
        ResultField::ALL.into_iter().find(|field| field.name() == name)
    }

    /// The title shown to the evaluator and in the report.
    pub fn title(self) -> &'static str {
        match self {
            ResultField::Findings => "Findings",
            ResultField::Analysis => "Analysis",
            ResultField::Assessment => "Assessment and verdict proposition",
            ResultField::Vulnerabilities => "Vulnerabilities and notes",
        }
    }

    /// What the field holds, for the evaluator and for the AI.
    pub fn description(self) -> &'static str {
        match self {
            ResultField::Findings => "What was found in the evidence: observations, deviations, open points.",
            ResultField::Analysis => "How the evidence was analysed against the requirement, and why.",
            ResultField::Assessment => {
                "The assessment of the requirement and the proposed verdict, in the terms of the standard."
            }
            ResultField::Vulnerabilities => {
                "The vulnerabilities that may follow from the solution used, and further notes."
            }
        }
    }
}

/// How a result came about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultOrigin {
    /// Written by the evaluator.
    Evaluator,
    /// Proposed by the AI and accepted by the evaluator unchanged.
    Ai,
    /// Proposed by the AI, corrected and accepted by the evaluator.
    AiCorrected,
}

impl ResultOrigin {
    pub fn name(self) -> &'static str {
        match self {
            ResultOrigin::Evaluator => "evaluator",
            ResultOrigin::Ai => "ai",
            ResultOrigin::AiCorrected => "ai_corrected",
        }
    }

    pub fn from_name(name: &str) -> Option<ResultOrigin> {
        [ResultOrigin::Evaluator, ResultOrigin::Ai, ResultOrigin::AiCorrected]
            .into_iter()
            .find(|origin| origin.name() == name)
    }
}

/// The accepted text of one field of a requirement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequirementResult {
    /// The requirement (a tile of the evaluation).
    pub tile: i64,
    pub field: ResultField,
    pub text: String,
    pub origin: ResultOrigin,
}

/// A text of the AI for one field of a requirement, waiting for the
/// evaluator: shown marked as AI, never used as a result until it is
/// accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposal {
    pub id: i64,
    pub tile: i64,
    pub field: ResultField,
    pub text: String,
    /// The model that wrote it, as the AI connection named it.
    pub model: String,
    /// When it was made, in seconds since 1970 (UTC).
    pub created: i64,
}

/// Who did a step of the journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Actor {
    /// The evaluator (the one user of the installation).
    Evaluator,
    /// An AI model, by the name its connection gave.
    Model(String),
}

/// What a step of the journal was.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalAction {
    /// The test project was imported; the detail names it and what
    /// was left out.
    Imported,
    /// The AI proposed a text; the detail is the text.
    Proposed,
    /// The evaluator accepted a proposal unchanged; the detail is the
    /// text.
    Accepted,
    /// The evaluator corrected a proposal and accepted it; the detail
    /// holds the proposal and the accepted text.
    Corrected,
    /// The evaluator rejected a proposal; the detail is its text.
    Rejected,
    /// The evaluator wrote a result; the detail holds the earlier and
    /// the new text.
    Written,
}

impl JournalAction {
    pub fn name(self) -> &'static str {
        match self {
            JournalAction::Imported => "imported",
            JournalAction::Proposed => "proposed",
            JournalAction::Accepted => "accepted",
            JournalAction::Corrected => "corrected",
            JournalAction::Rejected => "rejected",
            JournalAction::Written => "written",
        }
    }

    pub fn from_name(name: &str) -> Option<JournalAction> {
        [
            JournalAction::Imported,
            JournalAction::Proposed,
            JournalAction::Accepted,
            JournalAction::Corrected,
            JournalAction::Rejected,
            JournalAction::Written,
        ]
        .into_iter()
        .find(|action| action.name() == name)
    }
}

/// One step of the journal of an evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalEntry {
    pub id: i64,
    /// In seconds since 1970 (UTC).
    pub time: i64,
    pub actor: Actor,
    pub action: JournalAction,
    /// The tile the step concerns, if any.
    pub tile: Option<i64>,
    /// The field of a result the step concerns, if any.
    pub field: Option<ResultField>,
    /// What was done, in words, with the texts involved.
    pub detail: String,
}

impl EvaluationFields {
    /// Checks the fields; the name against the other evaluations is
    /// checked by the storage.
    pub fn check(&self) -> Result<(), Invalid> {
        if self.name.trim().is_empty() {
            return Err(Invalid::new("name", "the evaluation needs a name"));
        }

        if self.product.trim().is_empty() {
            return Err(Invalid::new("product", "the evaluation needs the product it evaluates"));
        }

        check_text("name", &self.name, MAX_NAME, false)?;
        check_text("product", &self.product, MAX_SHORT, false)?;
        check_text("product version", &self.product_version, MAX_SHORT, false)?;
        check_text("manufacturer", &self.manufacturer, MAX_SHORT, false)?;
        check_text("evaluator", &self.evaluator, MAX_SHORT, false)
    }
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "evaluations_tests.rs"]
mod tests;
