//! An OSCAL mapping collection (new in OSCAL 1.2.0): relations between
//! the requirements of two documents, for example the work units of a
//! methodology and the evaluator actions of the criteria, or the
//! requirements of two standards.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::common::{BackMatter, Metadata, Other};

/// A mapping collection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct MappingCollection {
    pub uuid: String,
    pub metadata: Metadata,
    /// How the mappings were made (method, rationale, status, ...).
    pub provenance: Value,
    pub mappings: Vec<Mapping>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub back_matter: Option<BackMatter>,
    #[serde(flatten)]
    pub other: Other,
}

/// The relations from one document (the source) to another (the target).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Mapping {
    pub uuid: String,
    pub source_resource: Value,
    pub target_resource: Value,
    pub maps: Vec<Map>,
    #[serde(flatten)]
    pub other: Other,
}

/// One relation: "equivalent-to", "subset-of", "superset-of",
/// "intersects-with" or "no-relationship", between sources and targets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Map {
    pub uuid: String,
    pub relationship: String,
    pub sources: Vec<MapItem>,
    pub targets: Vec<MapItem>,
    #[serde(flatten)]
    pub other: Other,
}

/// An end of a relation: a control, a statement, an objective, ... by
/// its id.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct MapItem {
    #[serde(rename = "type")]
    pub kind: String,
    pub id_ref: String,
    #[serde(flatten)]
    pub other: Other,
}
