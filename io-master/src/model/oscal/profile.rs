//! An OSCAL profile: a selection of the controls of catalogs (or of
//! other profiles), with their parameters set. In the program: a
//! Protection Profile, the requirements of a Security Target, the
//! requirements chosen for one evaluation.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::common::{BackMatter, Metadata, Other};

/// A profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Profile {
    pub uuid: String,
    pub metadata: Metadata,
    pub imports: Vec<Import>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merge: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modify: Option<Modify>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub back_matter: Option<BackMatter>,
    #[serde(flatten)]
    pub other: Other,
}

/// What a profile takes from one catalog or profile (`href`: in the
/// program, `#` and the UUID of a resource of the back matter that
/// names the document).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Import {
    pub href: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include_all: Option<Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub include_controls: Vec<SelectControls>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude_controls: Vec<SelectControls>,
    #[serde(flatten)]
    pub other: Other,
}

/// Controls chosen by their ids.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SelectControls {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub with_child_controls: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub with_ids: Vec<String>,
    #[serde(flatten)]
    pub other: Other,
}

/// The changes a profile makes to what it takes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Modify {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub set_parameters: Vec<SetParameter>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alters: Vec<Value>,
    #[serde(flatten)]
    pub other: Other,
}

/// A parameter set: in a Security Target, an operation of a requirement
/// completed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SetParameter {
    pub param_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<String>,
    #[serde(flatten)]
    pub other: Other,
}
