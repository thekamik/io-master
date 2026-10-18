//! An OSCAL assessment plan: the evaluation plan (function a of
//! docs/oscal.md): the controls and objectives reviewed, the subjects,
//! the tools, and the tasks with their order. Typed further in step
//! 41.4; until then its parts are kept as they come.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::common::{BackMatter, Link, Metadata, Other, Property};

/// An assessment plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AssessmentPlan {
    pub uuid: String,
    pub metadata: Metadata,
    /// The system security plan of what is assessed.
    pub import_ssp: Value,
    pub reviewed_controls: Value,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tasks: Vec<Task>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub back_matter: Option<BackMatter>,
    #[serde(flatten)]
    pub other: Other,
}

/// A task of the plan, with the tasks it depends on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Task {
    pub uuid: String,
    /// "milestone" or "action".
    #[serde(rename = "type")]
    pub kind: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub props: Vec<Property>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<Link>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tasks: Vec<Task>,
    #[serde(flatten)]
    pub other: Other,
}
