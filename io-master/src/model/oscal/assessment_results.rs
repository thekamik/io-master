//! OSCAL assessment results: observations (the evidence), findings per
//! objective, risks, and the log of the assessment. Imported, checked,
//! kept and exported as a document (docs/oscal.md, section 1); the
//! evaluation of the program is its own model (the Evaluation lab), so
//! its parts are kept as they come.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::common::{BackMatter, Metadata, Other};

/// Assessment results.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AssessmentResults {
    pub uuid: String,
    pub metadata: Metadata,
    /// The assessment plan the results follow.
    pub import_ap: Value,
    pub results: Vec<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub back_matter: Option<BackMatter>,
    #[serde(flatten)]
    pub other: Other,
}
