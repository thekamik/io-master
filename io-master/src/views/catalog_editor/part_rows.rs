//! The parts of a group or a control (statement, items, guidance, ...) as
//! rows with their level, so that the page shows them without recursion:
//! for the object chosen and for the proposals of the AI.

use serde::Serialize;

use crate::app::error::UiError;
use crate::model::oscal::catalog::tree::flat_parts;
use crate::model::oscal::common::Part;

/// A part, as a row.
#[derive(Debug, Serialize)]
pub struct PartRowView {
    /// The level, 0 for a part of the object.
    pub level: usize,
    pub name: String,
    pub id: Option<String>,
    pub title: Option<String>,
    pub prose: Option<String>,
}

/// The rows of `parts` and the parts in them, in the order of the document.
pub fn rows(parts: &[Part]) -> Result<Vec<PartRowView>, UiError> {
    let flat = match flat_parts(parts) {
        Ok(flat) => flat,
        Err(error) => return Err(UiError::Invalid(error.to_string())),
    };

    let mut rows = Vec::with_capacity(flat.len());

    for (level, part) in flat {
        rows.push(PartRowView {
            level,
            name: part.name.clone(),
            id: part.id.clone(),
            title: part.title.clone(),
            prose: part.prose.clone(),
        });
    }

    Ok(rows)
}
