//! The documents of OSCAL as the page shows them in a list.

use serde::Serialize;

use crate::model::oscal::OscalSummary;

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

impl From<OscalSummary> for OscalDocumentView {
    fn from(summary: OscalSummary) -> OscalDocumentView {
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
