//! The data of a requirement of the laboratory, as the page sends it and
//! gets it back to be changed.

use serde::{Deserialize, Serialize};

use crate::model_old::own_requirement::OwnRequirement;

/// A requirement of the laboratory, as the page sends it.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnRequirementForm {
    pub identifier: String,
    pub title: String,
    pub statement: String,
}

impl OwnRequirementForm {
    /// The requirement of the model.
    pub fn requirement(&self) -> OwnRequirement {
        OwnRequirement {
            identifier: self.identifier.clone(),
            title: self.title.clone(),
            statement: self.statement.clone(),
        }
    }
}

/// A requirement of the laboratory, as the page gets it to change it.
#[derive(Debug, Serialize)]
pub struct OwnRequirementView {
    pub identifier: String,
    pub title: String,
    pub statement: String,
}

impl OwnRequirementView {
    pub fn new(requirement: OwnRequirement) -> OwnRequirementView {
        OwnRequirementView {
            identifier: requirement.identifier,
            title: requirement.title,
            statement: requirement.statement,
        }
    }
}
