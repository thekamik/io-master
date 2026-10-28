//! The data of the form of a master project, as the page gets it: what the
//! planner wrote, the catalogs it uses, and those it could use.

use serde::{Deserialize, Serialize};

use crate::model_old::evaluation_project::ProjectFields;

use super::super::new_project::view::CatalogChoiceView;

/// A master project in its form.
#[derive(Debug, Serialize)]
pub struct ProjectView {
    pub uuid: String,
    pub title: String,
    pub description: String,
    /// The catalogs it uses, in the order chosen.
    pub catalogs: Vec<CatalogChoiceView>,
    /// The finished catalogs it could use.
    pub available: Vec<CatalogChoiceView>,
    /// The agent with knowledge of its works of the AI (step 57.6); `None`: none.
    pub agent: Option<i64>,
}

/// What the page sends of a project.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectForm {
    pub title: String,
    pub description: String,
    pub catalogs: Vec<String>,
}

impl ProjectForm {
    /// The fields of the model.
    pub fn fields(&self) -> ProjectFields {
        ProjectFields { title: self.title.clone(), description: self.description.clone() }
    }
}
