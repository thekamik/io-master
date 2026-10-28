//! An OSCAL catalog: the requirements (controls) of a standard, in
//! groups, with their parts and parameters. In the program: the criteria
//! of a standard, a methodology with its work units, a test
//! specification (docs/oscal.md, section 3).
//!
//! ```text
//! catalog.rs        the types (this file), the error of a change
//! catalog/tree.rs   where an object is, the walk through all of them (no recursion)
//! catalog/edit.rs   a group or a control added, changed, removed
//! ```

pub mod edit;
pub mod tree;

use std::fmt;

use serde::{Deserialize, Serialize};

use super::common::{BackMatter, Link, Metadata, Other, Parameter, Part, Property};
use tree::{MAX_DEPTH, MAX_OBJECTS};

/// A catalog.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Catalog {
    pub uuid: String,
    pub metadata: Metadata,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<Parameter>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub controls: Vec<Control>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<Group>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub back_matter: Option<BackMatter>,
    #[serde(flatten)]
    pub other: Other,
}

/// A group of controls, or of groups (a class or a family of a
/// standard, a section).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Group {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    pub title: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<Parameter>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub props: Vec<Property>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<Link>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<Part>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<Group>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub controls: Vec<Control>,
    #[serde(flatten)]
    pub other: Other,
}

/// A control: one requirement, with its statement, its parameters, and
/// its own controls (enhancements).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Control {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    pub title: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<Parameter>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub props: Vec<Property>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<Link>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<Part>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub controls: Vec<Control>,
    #[serde(flatten)]
    pub other: Other,
}

impl Catalog {
    /// A new catalog without groups, controls and sources: `uuid`, `title`,
    /// the version of its standard (`version`, empty: "1"), changed at
    /// `last_modified` (a time of OSCAL).
    ///
    /// Called by: `views/converter/new_catalog/made.rs` (a catalog to be
    /// converted) and `views/evaluation_plan/own_requirement/catalog.rs` (the
    /// catalog of the requirements of the laboratory).
    pub fn empty(uuid: String, title: &str, version: &str, last_modified: String) -> Catalog {
        Catalog {
            uuid,
            metadata: Metadata {
                title: title.to_owned(),
                published: None,
                last_modified,
                version: if version.is_empty() { "1".to_owned() } else { version.to_owned() },
                oscal_version: super::OSCAL_VERSION.to_owned(),
                props: Vec::new(),
                links: Vec::new(),
                remarks: None,
                other: Other::default(),
            },
            params: Vec::new(),
            controls: Vec::new(),
            groups: Vec::new(),
            back_matter: None,
            other: Other::default(),
        }
    }
}

impl Control {
    /// The label of the control: the identifier the standard writes
    /// (`ADV_FSP.4`), kept by the program as the property `label`; `None`
    /// when it has none.
    ///
    /// Called by: the views that show a requirement (`views/evaluation_plan/`).
    pub fn label(&self) -> Option<&str> {
        for prop in &self.props {
            if prop.name == "label" {
                return Some(&prop.value);
            }
        }

        None
    }
}

/// Why a change of a catalog was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogError {
    /// There is no group or control with this id where it was looked for.
    NoSuchObject(String),
    /// Another group or control of the catalog has this id.
    IdTaken(String),
    /// What was given breaks a rule; the reason, for the evaluator.
    Invalid(String),
    /// Groups and enhancements deeper than `tree::MAX_DEPTH` levels.
    TooDeep,
    /// More groups and controls than `tree::MAX_OBJECTS`.
    TooLarge,
}

impl fmt::Display for CatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CatalogError::NoSuchObject(id) => write!(formatter, "the catalog has no group or control {id}"),
            CatalogError::IdTaken(id) => write!(formatter, "another group or control of the catalog has the id {id}"),
            CatalogError::Invalid(reason) => formatter.write_str(reason),
            CatalogError::TooDeep => write!(formatter, "the catalog is deeper than {MAX_DEPTH} levels"),
            CatalogError::TooLarge => write!(formatter, "the catalog has more than {MAX_OBJECTS} groups and controls"),
        }
    }
}

impl std::error::Error for CatalogError {}
