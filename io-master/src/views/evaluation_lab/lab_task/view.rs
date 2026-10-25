//! The data of the screen of a task of an evaluation, as the page gets it
//! and sends it: the scheme (read only), the requirement as its catalog
//! writes it, the related elements, every input with its evidence, chat and
//! fragments, the results with what the AI proposed, every output with its
//! text and whether its condition holds; the agent with knowledge of the
//! task and its passages (step 57.4).

use serde::{Deserialize, Serialize};

use crate::app::error::AppError;
use crate::components::part_rows::view::PartRowView;
use crate::model::ai::{Speaker, Turn};
use crate::model::evaluation::{Evidence, Fragment, Results, Verdict};
use crate::model::task_scheme::Block;

use super::super::lab_tasks::view::AgentOptionView;

/// A block of the scheme.
#[derive(Debug, Serialize)]
pub struct LabBlockView {
    pub uuid: String,
    pub kind: &'static str,
    pub x: i64,
    pub y: i64,
    pub name: String,
    pub evidence: String,
    pub instructions: String,
    pub target: String,
    pub format: &'static str,
    pub condition: &'static str,
}

impl LabBlockView {
    pub fn new(block: Block) -> LabBlockView {
        LabBlockView {
            uuid: block.uuid,
            kind: block.kind.name(),
            x: block.x,
            y: block.y,
            name: block.fields.name,
            evidence: block.fields.evidence,
            instructions: block.fields.instructions,
            target: block.fields.target,
            format: block.fields.format.name(),
            condition: block.fields.condition.name(),
        }
    }
}

/// A connection of two blocks.
#[derive(Debug, Serialize)]
pub struct LabConnectionView {
    pub from: String,
    pub to: String,
}

/// The requirement as its catalog writes it.
#[derive(Debug, Serialize)]
pub struct LabRequirementView {
    pub label: String,
    pub title: String,
    pub catalog_title: String,
    pub parts: Vec<PartRowView>,
}

/// A related element.
#[derive(Debug, Serialize)]
pub struct LabRelatedView {
    pub label: String,
    pub title: String,
}

/// A piece of evidence, without its text.
#[derive(Debug, Serialize)]
pub struct EvidenceView {
    pub uuid: String,
    /// `"pdf"`, `"text"` or `"note"`.
    pub kind: &'static str,
    pub name: String,
    pub pages: usize,
    pub added: i64,
}

impl EvidenceView {
    pub fn new(evidence: &Evidence) -> EvidenceView {
        EvidenceView {
            uuid: evidence.uuid.clone(),
            kind: evidence.kind.name(),
            name: evidence.name.clone(),
            pages: evidence.pages.len(),
            added: evidence.added,
        }
    }
}

/// A message of the chat of an input.
#[derive(Debug, Serialize)]
pub struct MessageView {
    /// `"evaluator"` or `"model"`.
    pub speaker: &'static str,
    pub text: String,
}

impl MessageView {
    pub fn new(turn: Turn) -> MessageView {
        let speaker = match turn.speaker {
            Speaker::Evaluator => "evaluator",
            Speaker::Model => "model",
        };

        MessageView { speaker, text: turn.text }
    }
}

/// A fragment of the evidence of an input.
#[derive(Debug, Serialize)]
pub struct FragmentView {
    pub uuid: String,
    pub evidence: String,
    pub page: Option<u32>,
    pub quote: String,
    pub note: String,
    pub found: bool,
    pub accepted: bool,
    pub by_ai: bool,
}

impl FragmentView {
    pub fn new(fragment: Fragment) -> FragmentView {
        FragmentView {
            uuid: fragment.uuid,
            evidence: fragment.evidence,
            page: fragment.page,
            quote: fragment.quote,
            note: fragment.note,
            found: fragment.found,
            accepted: fragment.accepted,
            by_ai: fragment.by_ai,
        }
    }
}

/// An input with what the evaluator delivered and found.
#[derive(Debug, Serialize)]
pub struct InputView {
    pub block: String,
    pub evidence: Vec<EvidenceView>,
    pub messages: Vec<MessageView>,
    pub fragments: Vec<FragmentView>,
}

/// The four results and the verdict.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultsForm {
    pub findings: String,
    pub analysis: String,
    pub assessment: String,
    pub vulnerabilities: String,
    /// `"met"`, `"not_met"`, `"inconclusive"`, or none.
    pub verdict: Option<String>,
}

impl ResultsForm {
    pub fn new(results: Results) -> ResultsForm {
        ResultsForm {
            findings: results.findings,
            analysis: results.analysis,
            assessment: results.assessment,
            vulnerabilities: results.vulnerabilities,
            // A short closure: Option::map, which clippy asks for here.
            verdict: results.verdict.map(|verdict| verdict.name().to_owned()),
        }
    }

    /// The results of the model; refused when the verdict is one the
    /// program does not know.
    pub fn results(&self) -> Result<Results, AppError> {
        let verdict = match &self.verdict {
            Some(name) => match Verdict::from_name(name) {
                Some(verdict) => Some(verdict),
                None => return Err(AppError::Invalid("there is no such verdict".to_owned())),
            },
            None => None,
        };

        Ok(Results {
            findings: self.findings.clone(),
            analysis: self.analysis.clone(),
            assessment: self.assessment.clone(),
            vulnerabilities: self.vulnerabilities.clone(),
            verdict,
        })
    }
}

/// What the AI proposed for the results, with its reason.
#[derive(Debug, Serialize)]
pub struct ProposalView {
    pub results: ResultsForm,
    pub reason: String,
}

/// An output with its text.
#[derive(Debug, Serialize)]
pub struct OutputView {
    pub block: String,
    pub text: String,
    pub proposal: Option<String>,
    /// Whether its condition holds for the verdict set.
    pub applies: bool,
}

/// A passage of the knowledge of the agent of a task, ticked when it goes
/// into the question of the assessment (step 57.4).
#[derive(Debug, Serialize)]
pub struct ContextPassageView {
    pub id: i64,
    pub source: String,
    pub page: Option<u32>,
    pub label: String,
    pub text: String,
    pub chosen: bool,
}

/// The agent with knowledge of a task: how it is chosen (`"evaluation"`,
/// `"none"`, `"agent"`), the agent of the task's own, the name of the agent
/// in force, the agents to choose, and its passages.
#[derive(Debug, Serialize)]
pub struct KnowledgeView {
    pub choice: &'static str,
    pub agent: Option<i64>,
    pub used: Option<String>,
    pub agents: Vec<AgentOptionView>,
    pub passages: Vec<ContextPassageView>,
}

/// The screen of a task.
#[derive(Debug, Serialize)]
pub struct LabTaskScreenView {
    pub uuid: String,
    /// `"todo"`, `"working"` or `"done"`.
    pub state: &'static str,
    pub requirement: LabRequirementView,
    pub related: Vec<LabRelatedView>,
    pub blocks: Vec<LabBlockView>,
    pub connections: Vec<LabConnectionView>,
    pub inputs: Vec<InputView>,
    pub results: ResultsForm,
    pub proposal: Option<ProposalView>,
    pub outputs: Vec<OutputView>,
    /// The titles of the four results.
    pub result_titles: [&'static str; 4],
    pub knowledge: KnowledgeView,
}
