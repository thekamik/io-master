//! The data of the search of a knowledge base, as the page gets it.

use serde::Serialize;

use crate::model::knowledge::Hit;

/// A passage found, with its source, where it stands, and its score.
#[derive(Debug, Serialize)]
pub struct HitView {
    pub id: i64,
    pub source: String,
    pub page: Option<u32>,
    pub label: String,
    pub text: String,
    /// BM25 of SQLite: the lower, the better the passage fits.
    pub score: f64,
}

/// The passages found, as the page shows them.
///
/// Called by: `knowledge_search_run` (`knowledge.rs`).
pub fn all(hits: Vec<Hit>) -> Vec<HitView> {
    let mut views = Vec::with_capacity(hits.len());

    for hit in hits {
        views.push(HitView {
            id: hit.id,
            source: hit.source,
            page: hit.passage.page,
            label: hit.passage.label,
            text: hit.passage.text,
            score: hit.score,
        });
    }

    views
}
