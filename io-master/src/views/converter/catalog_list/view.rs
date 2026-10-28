//! The data of the list of catalogs of the view Converter, as the page gets
//! it: the catalogs of OSCAL of the evaluator, each opened in the editor of a
//! catalog. A new catalog (`new_catalog/`) is given back the same way.

use serde::Serialize;

use crate::model_old::oscal::{OscalModel, OscalSummary};

/// A catalog of OSCAL, as a row of the list.
#[derive(Debug, Clone, Serialize)]
pub struct CatalogView {
    pub uuid: String,
    pub title: String,
    pub version: String,
    pub last_modified: String,
}

impl CatalogView {
    /// The view of one stored catalog.
    ///
    /// Called by: [`catalogs`], and `new_catalog_create` (`converter.rs`).
    pub fn new(summary: OscalSummary) -> CatalogView {
        CatalogView {
            uuid: summary.uuid,
            title: summary.title,
            version: summary.version,
            last_modified: summary.last_modified,
        }
    }
}

/// The catalogs among `summaries` (the documents of OSCAL of every model),
/// in their order.
///
/// Called by: `catalog_list_get` (`converter.rs`).
pub fn catalogs(summaries: Vec<OscalSummary>) -> Vec<CatalogView> {
    let mut views = Vec::with_capacity(summaries.len());

    for summary in summaries {
        if summary.model == OscalModel::Catalog {
            views.push(CatalogView::new(summary));
        }
    }

    views
}
