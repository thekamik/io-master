//! The data of a new master project, as the page gets it: the catalogs it
//! can use (the finished ones).

use serde::Serialize;

use crate::model::oscal::OscalSummary;

/// A catalog a project can use.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CatalogChoiceView {
    pub uuid: String,
    pub title: String,
    pub version: String,
}

impl CatalogChoiceView {
    pub fn new(summary: &OscalSummary) -> CatalogChoiceView {
        CatalogChoiceView { uuid: summary.uuid.clone(), title: summary.title.clone(), version: summary.version.clone() }
    }
}
