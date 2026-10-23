//! The place of a new object, as the page sends it.

use serde::Deserialize;

use crate::app::error::UiError;
use crate::model::oscal::catalog::edit::Place;

/// Where a new object goes: `{"kind": "top"}`, `{"kind": "group", "id":
/// "ADV"}`, `{"kind": "control", "id": "ADV_FSP.1"}`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlaceForm {
    /// "top", "group" or "control".
    pub kind: String,
    #[serde(default)]
    pub id: Option<String>,
}

impl PlaceForm {
    pub fn into_place(self) -> Result<Place, UiError> {
        match (self.kind.as_str(), self.id) {
            ("top", _) => Ok(Place::Top),
            ("group", Some(id)) => Ok(Place::Group(id)),
            ("control", Some(id)) => Ok(Place::Control(id)),
            _ => Err(UiError::Invalid("the place of the object is not known".to_owned())),
        }
    }
}
