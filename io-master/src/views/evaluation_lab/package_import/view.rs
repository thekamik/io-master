//! The data of the import of a work package, as the page sends it: the
//! product the evaluation is for.

use serde::Deserialize;

use crate::model::evaluation::EvaluationFields;

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
