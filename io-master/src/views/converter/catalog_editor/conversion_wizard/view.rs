//! The data of the wizard of a conversion, as the page gets it and sends
//! it: the stage, the pages of the table of contents, the groups with their
//! pages and state, and what is left to check in the review.

use serde::{Deserialize, Serialize};

use crate::model::conversion::Pages;

/// Pages of a document, as the page gets them.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct PagesView {
    pub document_id: i64,
    pub first_page: u32,
    pub last_page: u32,
}

impl PagesView {
    pub fn new(pages: Pages) -> PagesView {
        PagesView { document_id: pages.document_id, first_page: pages.first, last_page: pages.last }
    }
}

/// Pages of a document, as the page sends them.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PagesForm {
    pub document_id: i64,
    pub first_page: u32,
    pub last_page: u32,
}

/// A group whose controls the AI reads.
#[derive(Debug, Serialize)]
pub struct GroupView {
    pub id: String,
    pub title: String,
    /// Its pages; `None` when its start was not found and the evaluator has
    /// not given them.
    pub pages: Option<PagesView>,
    /// `"todo"`, `"proposed"` or `"done"`.
    pub state: &'static str,
    /// The controls it holds now.
    pub controls: usize,
}

/// What is left to check before the conversion is finished.
#[derive(Debug, Serialize)]
pub struct ReviewView {
    /// Proposals of the AI the evaluator has not decided.
    pub proposals: usize,
    /// Quotes not found on the page they name.
    pub quotes_not_found: usize,
    /// Groups without a control.
    pub groups_without_controls: usize,
}

/// The wizard of a conversion.
#[derive(Debug, Serialize)]
pub struct WizardView {
    /// `"contents"`, `"groups"`, `"controls"`, `"review"` or `"finished"`;
    /// `"finished"` also for a catalog that was imported, not converted.
    pub stage: &'static str,
    pub contents: Option<PagesView>,
    pub groups: Vec<GroupView>,
    pub review: ReviewView,
    /// The agent with knowledge of the works of the AI of the catalog (step 57.6); `None`: none.
    pub agent: Option<i64>,
}
