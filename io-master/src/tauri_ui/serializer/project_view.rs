//! The test projects and the list of their schemes as the page shows
//! and sends them (steps 25 to 27 of `docs/todo.md`).

use serde::{Deserialize, Serialize};

use crate::business::test_projects::checks::Finding;
use crate::model::test_projects::ProjectData;
use crate::model::test_projects::{PageSpan, Project, ProjectFields, ProposalState, Scheme, SchemeFields, TileKind};

use super::super::error::UiError;
use super::standard_view::DocumentView;

/// A project in the list.
#[derive(Debug, Clone, Serialize)]
pub struct ProjectView {
    pub id: i64,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub documents: Vec<i64>,
    pub ready: bool,
}

impl From<Project> for ProjectView {
    fn from(project: Project) -> ProjectView {
        ProjectView {
            id: project.id,
            name: project.fields.name,
            version: project.fields.version,
            description: project.fields.description,
            author: project.fields.author,
            documents: project.fields.documents,
            ready: project.ready,
        }
    }
}

/// The fields of a project, as the page sends them.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectForm {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub documents: Vec<i64>,
}

impl From<ProjectForm> for ProjectFields {
    fn from(form: ProjectForm) -> ProjectFields {
        ProjectFields {
            name: form.name,
            version: form.version,
            description: form.description,
            author: form.author,
            documents: form.documents,
        }
    }
}

/// The pages of a scheme.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageSpanView {
    pub document_id: i64,
    pub first_page: u32,
    pub last_page: u32,
}

impl From<PageSpan> for PageSpanView {
    fn from(pages: PageSpan) -> PageSpanView {
        PageSpanView { document_id: pages.document_id, first_page: pages.first_page, last_page: pages.last_page }
    }
}

/// The fields of a scheme, as the page sends them.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchemeForm {
    pub number: String,
    pub name: String,
    pub pages: Option<PageSpanView>,
    /// `"proposed"` or `"accepted"`.
    pub state: String,
}

impl SchemeForm {
    /// The fields of the core; the name of the state is checked here,
    /// everything else by the rules of the core.
    pub fn into_fields(self) -> Result<SchemeFields, UiError> {
        Ok(SchemeFields {
            number: self.number,
            name: self.name,
            pages: self.pages.map(|pages| PageSpan {
                document_id: pages.document_id,
                first_page: pages.first_page,
                last_page: pages.last_page,
            }),
            state: ProposalState::from_name(&self.state)
                .ok_or_else(|| UiError::Invalid("a scheme is proposed or accepted".to_owned()))?,
        })
    }
}

/// A scheme in the list of its project, with what is in it.
#[derive(Debug, Clone, Serialize)]
pub struct SchemeSummaryView {
    pub id: i64,
    pub number: String,
    pub name: String,
    /// `"proposed"` or `"accepted"`.
    pub state: &'static str,
    pub pages: Option<PageSpanView>,
    /// How many tiles, requirements, findings of the checks and tiles
    /// proposed by the AI it has.
    pub tiles: usize,
    pub requirements: usize,
    pub findings: usize,
    pub proposed: usize,
}

/// The summaries of the schemes of `data`, in its order; `findings`
/// are the findings of the whole project.
pub fn summaries(data: &ProjectData, findings: &[Finding]) -> Vec<SchemeSummaryView> {
    data.schemes.iter().map(|scheme| summary(data, findings, scheme)).collect()
}

/// The summary of one scheme of `data`.
pub fn summary(data: &ProjectData, findings: &[Finding], scheme: &Scheme) -> SchemeSummaryView {
    let tiles: Vec<_> = data.tiles.iter().filter(|tile| tile.scheme == scheme.id).collect();

    SchemeSummaryView {
        id: scheme.id,
        number: scheme.fields.number.clone(),
        name: scheme.fields.name.clone(),
        state: scheme.fields.state.name(),
        pages: scheme.fields.pages.map(PageSpanView::from),
        tiles: tiles.len(),
        requirements: tiles.iter().filter(|tile| tile.fields.kind == TileKind::Requirement).count(),
        findings: findings.iter().filter(|finding| tiles.iter().any(|tile| tile.id == finding.tile)).count(),
        proposed: tiles.iter().filter(|tile| tile.fields.state == ProposalState::Proposed).count(),
    }
}

/// An open project: its fields and the list of its schemes.
#[derive(Debug, Serialize)]
pub struct ProjectDataView {
    pub project: ProjectView,
    pub schemes: Vec<SchemeSummaryView>,
    /// The documents of the project.
    pub documents: Vec<DocumentView>,
    /// The findings of the checks in the whole project.
    pub findings: usize,
    /// Whether "Undo" has something to undo.
    pub can_undo: bool,
}

impl ProjectDataView {
    pub fn new(
        data: ProjectData,
        findings: &[Finding],
        documents: Vec<DocumentView>,
        can_undo: bool,
    ) -> ProjectDataView {
        ProjectDataView {
            schemes: summaries(&data, findings),
            findings: findings.len(),
            project: data.project.into(),
            documents,
            can_undo,
        }
    }
}
