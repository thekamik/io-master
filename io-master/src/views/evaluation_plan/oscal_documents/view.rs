//! The data of the list of documents of OSCAL, as the page gets it.

use serde::Serialize;

use crate::model_old::oscal::OscalSummary;

/// A stored document of OSCAL.
#[derive(Debug, Clone, Serialize)]
pub struct OscalDocumentView {
    pub uuid: String,
    /// The name of its model, as OSCAL writes it ("catalog", ...).
    pub model: &'static str,
    /// The name of its model for the evaluator ("Catalog", ...).
    pub model_title: &'static str,
    pub title: String,
    pub version: String,
    pub oscal_version: String,
    pub last_modified: String,
}

impl OscalDocumentView {
    pub fn new(summary: OscalSummary) -> OscalDocumentView {
        OscalDocumentView {
            uuid: summary.uuid,
            model: summary.model.name(),
            model_title: summary.model.title(),
            title: summary.title,
            version: summary.version,
            oscal_version: summary.oscal_version,
            last_modified: summary.last_modified,
        }
    }
}

/// The views of `summaries`, in their order, without the catalogs still in
/// conversion (`unfinished`, their UUIDs): the view Evaluation plan works
/// only on what the converter finished (user, 2026-10-24).
///
/// Called by: `oscal_documents_get` (`evaluation_plan.rs`).
pub fn all(summaries: Vec<OscalSummary>, unfinished: &[String]) -> Vec<OscalDocumentView> {
    let mut views = Vec::with_capacity(summaries.len());

    for summary in summaries {
        if !unfinished.contains(&summary.uuid) {
            views.push(OscalDocumentView::new(summary));
        }
    }

    views
}
