//! The data of the analysis of a standard with the AI (steps 20 and
//! 27 of `docs/todo.md`): a part of the pages sent in one question,
//! and what the model proposed, read from its answer (`ai/tasks/`).
//! Nothing proposed is taken on trust: the proposals are checked and
//! applied by `test_projects/analysis/`.

use crate::model::test_projects::TileFields;

/// A part of the text of one document, for one request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    pub document_id: i64,
    pub first_page: u32,
    pub last_page: u32,
    /// The pages, each after a line "=== Page N ===".
    pub text: String,
}

/// A tile as the model proposed it.
#[derive(Debug, Clone, PartialEq)]
pub struct ProposedTile {
    pub key: String,
    pub fields: TileFields,
}

/// The answer of the model to the task `chapter`, read.
#[derive(Debug, Clone, PartialEq)]
pub struct ChapterProposal {
    pub tiles: Vec<ProposedTile>,
    /// From and to: a key of the answer, or an identifier of a known
    /// tile.
    pub links: Vec<(String, String)>,
    /// What could not be read, and why.
    pub dropped: Vec<String>,
}

/// A chapter as the model proposed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposedChapter {
    pub number: String,
    pub title: String,
    /// The page printed in the table of contents, if it gives one.
    pub printed_page: Option<u32>,
}
