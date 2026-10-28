//! The data of the list of knowledge bases, as the page gets and sends it.

use serde::{Deserialize, Serialize};

use crate::model_old::knowledge::BaseSummary;

/// A base as a row of the list, with the numbers of its sources and passages.
#[derive(Debug, Serialize)]
pub struct BaseSummaryView {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub sources: usize,
    pub passages: usize,
}

/// The name and the description of a base, as the evaluator writes them.
#[derive(Debug, Deserialize)]
pub struct BaseForm {
    pub name: String,
    pub description: String,
}

/// The bases as the page shows them.
///
/// Called by: the commands of the list (`knowledge.rs`).
pub fn all(summaries: Vec<BaseSummary>) -> Vec<BaseSummaryView> {
    let mut views = Vec::with_capacity(summaries.len());

    for summary in summaries {
        views.push(BaseSummaryView {
            id: summary.base.id,
            name: summary.base.name,
            description: summary.base.description,
            sources: summary.sources,
            passages: summary.passages,
        });
    }

    views
}
