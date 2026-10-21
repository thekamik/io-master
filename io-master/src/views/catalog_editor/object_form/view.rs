//! The data of the object chosen, as the page gets it.

use serde::Serialize;
use serde_json::Value;

use crate::model::oscal::catalog::tree::Location;
use crate::model::oscal::catalog::{Catalog, Control, Group};
use crate::tauri_ui::error::UiError;

use super::super::part_rows::{self, PartRowView};

/// A group or a control of the catalog.
#[derive(Debug, Serialize)]
pub struct ObjectView {
    /// `"group"` or `"control"`.
    pub kind: &'static str,
    pub id: String,
    pub title: String,
    /// Its parts, as rows.
    pub parts: Vec<PartRowView>,
    /// Its own fields as JSON of OSCAL: without the groups and controls it
    /// holds, which a save leaves as they are.
    pub fields: Value,
}

impl ObjectView {
    /// The object at `location` of `catalog`, whose id is `id`.
    pub fn new(catalog: &Catalog, location: &Location, id: &str) -> Result<ObjectView, UiError> {
        if location.is_group() {
            let Some(group) = catalog.group_at(&location.groups) else {
                return Err(UiError::Invalid(format!("the catalog has no group {id}")));
            };

            return group_view(group, id);
        }

        let Some(control) = catalog.control_at(location) else {
            return Err(UiError::Invalid(format!("the catalog has no control {id}")));
        };

        control_view(control)
    }
}

fn group_view(group: &Group, id: &str) -> Result<ObjectView, UiError> {
    let own = Group { groups: Vec::new(), controls: Vec::new(), ..group.clone() };

    Ok(ObjectView {
        kind: "group",
        id: id.to_owned(),
        title: group.title.clone(),
        parts: part_rows::rows(&group.parts)?,
        fields: json(&own)?,
    })
}

fn control_view(control: &Control) -> Result<ObjectView, UiError> {
    let own = Control { controls: Vec::new(), ..control.clone() };

    Ok(ObjectView {
        kind: "control",
        id: control.id.clone(),
        title: control.title.clone(),
        parts: part_rows::rows(&control.parts)?,
        fields: json(&own)?,
    })
}

/// An object as its JSON of OSCAL.
fn json<T: Serialize>(object: &T) -> Result<Value, UiError> {
    match serde_json::to_value(object) {
        Ok(value) => Ok(value),
        Err(error) => Err(UiError::Failed(error.to_string())),
    }
}
