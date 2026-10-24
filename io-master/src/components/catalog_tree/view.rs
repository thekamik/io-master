//! The data of the component "tree of a catalog", as the page gets it.

use serde::Serialize;

use crate::app::error::AppError;
use crate::model::oscal::catalog::Catalog;

/// A row of the tree: a group or a control.
#[derive(Debug, Serialize)]
pub struct RowView {
    /// `"group"` or `"control"`.
    pub kind: &'static str,
    /// The id; a group may have none (then it cannot be chosen).
    pub id: Option<String>,
    pub title: String,
    /// The level, 0 at the top of the catalog.
    pub level: usize,
}

/// The tree of a catalog.
#[derive(Debug, Serialize)]
pub struct CatalogTreeView {
    pub title: String,
    pub version: String,
    pub last_modified: String,
    /// Every group and control, in the order of the document.
    pub rows: Vec<RowView>,
}

impl CatalogTreeView {
    pub fn new(catalog: &Catalog) -> Result<CatalogTreeView, AppError> {
        let locations = match catalog.locations() {
            Ok(locations) => locations,
            Err(error) => return Err(AppError::Invalid(error.to_string())),
        };

        let mut rows = Vec::with_capacity(locations.len());

        for location in locations {
            let level = location.depth().saturating_sub(1);

            if location.is_group() {
                if let Some(group) = catalog.group_at(&location.groups) {
                    rows.push(RowView { kind: "group", id: group.id.clone(), title: group.title.clone(), level });
                }
            } else if let Some(control) = catalog.control_at(&location) {
                rows.push(RowView {
                    kind: "control",
                    id: Some(control.id.clone()),
                    title: control.title.clone(),
                    level,
                });
            }
        }

        Ok(CatalogTreeView {
            title: catalog.metadata.title.clone(),
            version: catalog.metadata.version.clone(),
            last_modified: catalog.metadata.last_modified.clone(),
            rows,
        })
    }
}
