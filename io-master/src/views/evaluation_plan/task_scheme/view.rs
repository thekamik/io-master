//! The data of the scheme of a task, as the page gets it and sends it: the
//! task, its requirement as the standard writes it (shown beside the
//! drawing), the blocks and their connections.

use serde::{Deserialize, Serialize};

use crate::app::error::AppError;
use crate::model::task_scheme::{
    Block, BlockFields, Connection, Correction, OutputCondition, OutputFormat, ResultText,
};

use super::super::standard_browser::view::RequirementView;
use super::super::task_list::view::TaskView;

/// A block of the scheme.
#[derive(Debug, Serialize)]
pub struct BlockView {
    pub uuid: String,
    /// `"requirement"`, `"input"` or `"output"`.
    pub kind: &'static str,
    pub x: i64,
    pub y: i64,
    pub name: String,
    pub evidence: String,
    pub instructions: String,
    pub target: String,
    /// `"text"` or `"latex"`.
    pub format: &'static str,
    /// `"always"`, `"not_met"` or `"met"`.
    pub condition: &'static str,
    /// The reason of the AI while the block is only proposed (drawn dashed).
    pub proposal: Option<String>,
    /// The correction the AI proposes for the block, waiting for the planner.
    pub correction: Option<CorrectionView>,
}

/// A correction of a block the AI proposes.
#[derive(Debug, Serialize)]
pub struct CorrectionView {
    pub name: String,
    pub evidence: String,
    pub instructions: String,
    pub reason: String,
}

impl BlockView {
    /// The block with the correction proposed for it, if any.
    pub fn new(block: Block, correction: Option<Correction>) -> BlockView {
        // A short closure: Option::map, which clippy asks for here.
        let correction = correction.map(|found| CorrectionView {
            name: found.name,
            evidence: found.evidence,
            instructions: found.instructions,
            reason: found.reason,
        });

        BlockView {
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
            proposal: block.proposal,
            correction,
        }
    }
}

/// A connection of two blocks.
#[derive(Debug, Serialize)]
pub struct ConnectionView {
    pub from: String,
    pub to: String,
}

impl ConnectionView {
    pub fn new(connection: Connection) -> ConnectionView {
        ConnectionView { from: connection.from, to: connection.to }
    }
}

/// The scheme of a task.
#[derive(Debug, Serialize)]
pub struct SchemeView {
    pub task: TaskView,
    /// The requirement as its catalog writes it, with its quotes.
    pub requirement: RequirementView,
    pub blocks: Vec<BlockView>,
    pub connections: Vec<ConnectionView>,
    /// The titles of the four text results of the requirement, which its
    /// block shows and the outputs get.
    pub results: Vec<&'static str>,
}

/// The titles of the four text results, in their order.
///
/// Called by: `scheme::read`.
pub fn result_titles() -> Vec<&'static str> {
    let mut titles = Vec::with_capacity(ResultText::ALL.len());

    for text in ResultText::ALL {
        titles.push(text.title());
    }

    titles
}

/// The fields of a block, as the page sends them.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlockForm {
    pub name: String,
    pub evidence: String,
    pub instructions: String,
    pub target: String,
    pub format: String,
    pub condition: String,
}

impl BlockForm {
    /// The fields of the model; refused when the format or the condition is
    /// one the program does not know.
    pub fn fields(&self) -> Result<BlockFields, AppError> {
        let Some(format) = OutputFormat::from_name(&self.format) else {
            return Err(AppError::Invalid("there is no such format of an output".to_owned()));
        };
        let Some(condition) = OutputCondition::from_name(&self.condition) else {
            return Err(AppError::Invalid("there is no such condition of an output".to_owned()));
        };

        Ok(BlockFields {
            name: self.name.clone(),
            evidence: self.evidence.clone(),
            instructions: self.instructions.clone(),
            target: self.target.clone(),
            format,
            condition,
        })
    }
}
