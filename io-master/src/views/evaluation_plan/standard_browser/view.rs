//! The data of the browser of the standards of a master project, as the
//! page gets it: a requirement with its parts and its quotes, and the hits
//! of a search.

use serde::Serialize;

use crate::components::part_rows::view::PartRowView;

use super::super::own_requirement::view::OwnRequirementView;

/// A passage of a document of a standard a requirement comes from.
#[derive(Debug, Serialize)]
pub struct QuoteView {
    pub document_id: i64,
    pub page: u32,
    pub text: String,
    /// Whether the program found it on its page.
    pub found: bool,
}

/// A requirement (a control) of a catalog of the project.
#[derive(Debug, Serialize)]
pub struct RequirementView {
    pub catalog_uuid: String,
    pub id: String,
    pub label: String,
    pub title: String,
    /// Its statement, guidance and objectives, as rows.
    pub parts: Vec<PartRowView>,
    pub quotes: Vec<QuoteView>,
    /// Whether it is a task of the project already.
    pub in_project: bool,
    /// For a requirement of the laboratory, its fields to be changed;
    /// `None` for a requirement of a standard.
    pub own: Option<OwnRequirementView>,
}

/// A requirement found by a search.
#[derive(Debug, Serialize)]
pub struct HitView {
    pub catalog_uuid: String,
    pub catalog_title: String,
    pub id: String,
    pub label: String,
    pub title: String,
}
