//! The data of the import of a work package, as the page sends it (the
//! product the evaluation is for) and gets it back (the evaluations and the
//! notices of the import).

use serde::{Deserialize, Serialize};

use crate::model_old::evaluation::EvaluationFields;

use super::super::evaluation_list::view::EvaluationSummaryView;

/// The evaluations after an import, and what the evaluator should know of it
/// (a catalog kept in the version stored already).
#[derive(Debug, Serialize)]
pub struct ImportedView {
    pub evaluations: Vec<EvaluationSummaryView>,
    pub notices: Vec<String>,
}

/// The product and its version, both optional.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportForm {
    pub product: String,
    pub product_version: String,
}

impl ImportForm {
    pub fn fields(&self) -> EvaluationFields {
        EvaluationFields { product: self.product.clone(), product_version: self.product_version.clone() }
    }
}
