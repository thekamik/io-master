//! The data of the list of agents, as the page gets it.

use serde::Serialize;

use crate::model_old::knowledge::AgentSummary;

/// An agent as a row of the list, with the number of its bases.
#[derive(Debug, Serialize)]
pub struct AgentSummaryView {
    pub id: i64,
    pub name: String,
    pub bases: usize,
}

/// The agents as the page shows them.
///
/// Called by: the commands of the list of agents (`knowledge.rs`).
pub fn all(summaries: Vec<AgentSummary>) -> Vec<AgentSummaryView> {
    let mut views = Vec::with_capacity(summaries.len());

    for summary in summaries {
        views.push(AgentSummaryView { id: summary.id, name: summary.name, bases: summary.bases });
    }

    views
}
