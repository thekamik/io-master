//! The pieces every model of OSCAL shares: the metadata of a document,
//! properties and links, the parts and parameters of a control, and the
//! back matter (the resources a document names: for the program, the
//! PDFs of a standard, by their SHA-256).
//!
//! Every struct keeps the fields the program does not type in `other`,
//! as they came (see `model/oscal.rs`).

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// The fields of an OSCAL object the program does not type, kept as
/// they came.
pub type Other = Map<String, Value>;

/// The metadata of a document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Metadata {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub published: Option<String>,
    /// RFC 3339 date and time of the last change.
    pub last_modified: String,
    /// The version of the document (of the standard it describes).
    pub version: String,
    /// The version of OSCAL the document follows.
    pub oscal_version: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub props: Vec<Property>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<Link>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remarks: Option<String>,
    #[serde(flatten)]
    pub other: Other,
}

/// A property: a name and a value, in the namespace of NIST unless
/// `ns` says another.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Property {
    pub name: String,
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ns: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    #[serde(flatten)]
    pub other: Other,
}

/// A link to another object or resource.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Link {
    pub href: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rel: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(flatten)]
    pub other: Other,
}

/// A part of a control (a statement, its items, guidance, an
/// assessment objective or method), with its prose and its own parts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Part {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ns: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub props: Vec<Property>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prose: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<Part>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<Link>,
    #[serde(flatten)]
    pub other: Other,
}

/// A parameter: a value the control leaves open (an assignment or a
/// selection of a standard), set by a profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Parameter {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub props: Vec<Property>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<Link>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub select: Option<Selection>,
    #[serde(flatten)]
    pub other: Other,
}

/// The choices of a parameter that is a selection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Selection {
    /// "one" or "one-or-more"; one when missing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub how_many: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub choice: Vec<String>,
}

/// The resources a document names.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct BackMatter {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resources: Vec<Resource>,
    #[serde(flatten)]
    pub other: Other,
}

/// A resource: a document outside (for the program: a PDF of a
/// standard, found by the SHA-256 of its file) or included.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Resource {
    pub uuid: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub props: Vec<Property>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rlinks: Vec<ResourceLink>,
    #[serde(flatten)]
    pub other: Other,
}

/// Where a resource is, with the hashes that identify its file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ResourceLink {
    pub href: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hashes: Vec<Hash>,
    #[serde(flatten)]
    pub other: Other,
}

/// A hash of a file: the algorithm ("SHA-256") and the value in hex.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Hash {
    pub algorithm: String,
    pub value: String,
}
