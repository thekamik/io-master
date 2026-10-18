//! An OSCAL catalog: the requirements (controls) of a standard, in
//! groups, with their parts and parameters. In the program: the criteria
//! of a standard, a methodology with its work units, a test
//! specification (docs/oscal.md, section 3).

use serde::{Deserialize, Serialize};

use super::common::{BackMatter, Link, Metadata, Other, Parameter, Part, Property};

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
    /// Every control of the catalog, in the order of the document (the
    /// groups and the enhancements walked through), with the path of
    /// titles of the groups it is in.
    pub fn all_controls(&self) -> Vec<(Vec<&str>, &Control)> {
        fn walk_control<'a>(path: &[&'a str], control: &'a Control, out: &mut Vec<(Vec<&'a str>, &'a Control)>) {
            out.push((path.to_vec(), control));

            for inner in &control.controls {
                walk_control(path, inner, out);
            }
        }

        fn walk_group<'a>(path: &[&'a str], group: &'a Group, out: &mut Vec<(Vec<&'a str>, &'a Control)>) {
            let mut path = path.to_vec();
            path.push(&group.title);

            for control in &group.controls {
                walk_control(&path, control, out);
            }

            for inner in &group.groups {
                walk_group(&path, inner, out);
            }
        }

        let mut out = Vec::new();

        for control in &self.controls {
            walk_control(&[], control, &mut out);
        }

        for group in &self.groups {
            walk_group(&[], group, &mut out);
        }

        out
    }
}
