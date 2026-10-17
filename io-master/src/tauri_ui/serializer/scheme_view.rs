//! A scheme of a test project as the drawing shows it (its tiles, the
//! tiles of other schemes it is linked to, the links), the fields of a
//! tile as the page sends them, and the events of a work with AI.
//!
//! A work with AI reaches the page through a channel of Tauri, as
//! these events:
//!
//! ```text
//! {"event": "scheme", "index": 2, "count": 9, "number": "9.1", "name": "..."}   a scheme starts
//! {"event": "part", "index": 1, "count": 12, "document": "...", "first_page": 1, "last_page": 9}
//! {"event": "writing", "characters": 1830}                the model is answering
//! {"event": "applied", "added": 7, "joined": 1, "links": 9, "dropped": ["..."]}
//! {"event": "chapters", "added": 14, "known": 0, "without_pages": 1, "dropped": []}   a table of contents read
//! {"event": "failed", "message": "..."}                   this part gave nothing
//! {"event": "end", "stopped": false}                      last
//! ```

use serde::{Deserialize, Serialize};

use crate::test_projects::checks::Finding;
use crate::test_projects::model::ProjectData;
use crate::test_projects::model::{External, OutputSection, ProposalState, Source, Tile, TileFields, TileKind};
use crate::test_projects::rules::LinkMeaning;

use super::super::error::UiError;
use super::project_view::{ProjectView, SchemeSummaryView, summaries, summary};
use super::standard_view::DocumentView;

/// A source of a tile.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceView {
    pub document_id: i64,
    pub page: u32,
    pub quote: String,
    /// Set by the program; ignored when the page sends a tile.
    #[serde(default)]
    pub found: bool,
}

/// An input described in a document not loaded.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalView {
    pub document: String,
    pub place: String,
}

/// A tile.
#[derive(Debug, Serialize)]
pub struct TileView {
    pub id: i64,
    #[serde(flatten)]
    pub fields: TileForm,
}

/// The fields of a tile, as the page shows and sends them.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TileForm {
    /// `"input"`, `"requirement"` or `"output"`.
    pub kind: String,
    pub identifier: String,
    pub title: String,
    pub description: String,
    pub place: String,
    /// `"proposed"` or `"accepted"`.
    pub state: String,
    pub external: Option<ExternalView>,
    /// The sections of the report of an output, by name.
    pub sections: Vec<String>,
    pub instructions: String,
    pub x: f64,
    pub y: f64,
    pub sources: Vec<SourceView>,
}

impl From<TileFields> for TileForm {
    fn from(fields: TileFields) -> TileForm {
        TileForm {
            kind: fields.kind.name().to_owned(),
            identifier: fields.identifier,
            title: fields.title,
            description: fields.description,
            place: fields.place,
            state: fields.state.name().to_owned(),
            external: fields
                .external
                .map(|external| ExternalView { document: external.document, place: external.place }),
            sections: fields.sections.iter().map(|section| section.name().to_owned()).collect(),
            instructions: fields.instructions,
            x: fields.x,
            y: fields.y,
            sources: fields
                .sources
                .into_iter()
                .map(|source| SourceView {
                    document_id: source.document_id,
                    page: source.page,
                    quote: source.quote,
                    found: source.found,
                })
                .collect(),
        }
    }
}

impl TileForm {
    /// The fields of the core; the names of kinds, states and sections
    /// are checked here, everything else by the rules of the core.
    pub fn into_fields(self) -> Result<TileFields, UiError> {
        let kind = TileKind::from_name(&self.kind)
            .ok_or_else(|| UiError::Invalid("a tile is an input, a requirement or an output".to_owned()))?;
        let state = ProposalState::from_name(&self.state)
            .ok_or_else(|| UiError::Invalid("a tile is proposed or accepted".to_owned()))?;

        let mut sections = Vec::new();

        for name in &self.sections {
            sections.push(
                OutputSection::from_name(name)
                    .ok_or_else(|| UiError::Invalid(format!("there is no section {name} of a report")))?,
            );
        }

        Ok(TileFields {
            kind,
            identifier: self.identifier,
            title: self.title,
            description: self.description,
            place: self.place,
            sources: self
                .sources
                .into_iter()
                .map(|source| Source {
                    document_id: source.document_id,
                    page: source.page,
                    quote: source.quote,
                    found: false,
                })
                .collect(),
            state,
            external: self.external.map(|external| External { document: external.document, place: external.place }),
            sections,
            instructions: self.instructions,
            x: self.x,
            y: self.y,
        })
    }
}

impl From<Tile> for TileView {
    fn from(tile: Tile) -> TileView {
        TileView { id: tile.id, fields: tile.fields.into() }
    }
}

/// A link, with its meaning from the kinds it joins.
#[derive(Debug, Serialize)]
pub struct LinkView {
    pub id: i64,
    pub from: i64,
    pub to: i64,
    /// `"needed_by"`, `"reported_in"`, `"provides"` or `"precedes"`.
    pub meaning: &'static str,
}

/// A tile of another scheme of the project, linked to a tile of the
/// scheme shown: drawn as a shadow, read only (step 26).
#[derive(Debug, Serialize)]
pub struct ShadowView {
    pub id: i64,
    /// The scheme it is in, and its number and name.
    pub scheme: i64,
    pub scheme_number: String,
    pub scheme_name: String,
    pub kind: &'static str,
    pub identifier: String,
    pub title: String,
}

/// A finding of the checks.
#[derive(Debug, Serialize)]
pub struct FindingView {
    pub kind: &'static str,
    pub tile: i64,
    pub message: &'static str,
}

impl From<Finding> for FindingView {
    fn from(finding: Finding) -> FindingView {
        FindingView { kind: finding.kind.name(), tile: finding.tile, message: finding.kind.sentence() }
    }
}

/// A section of the report of an output, for the form.
#[derive(Debug, Serialize)]
pub struct SectionView {
    pub name: &'static str,
    pub title: &'static str,
    pub description: &'static str,
}

/// An open scheme: everything the drawing needs.
#[derive(Debug, Serialize)]
pub struct SchemeDataView {
    pub project: ProjectView,
    pub scheme: SchemeSummaryView,
    /// Every scheme of the project, for moving tiles and linking to
    /// them.
    pub schemes: Vec<SchemeSummaryView>,
    /// The tiles of this scheme.
    pub tiles: Vec<TileView>,
    /// The tiles of other schemes linked to the tiles of this one.
    pub shadows: Vec<ShadowView>,
    /// The links with at least one end in this scheme.
    pub links: Vec<LinkView>,
    /// The findings of the tiles of this scheme.
    pub findings: Vec<FindingView>,
    /// The documents of the project.
    pub documents: Vec<DocumentView>,
    /// The sections an output can have.
    pub sections: Vec<SectionView>,
    /// Whether "Undo" has something to undo (in the whole project).
    pub can_undo: bool,
}

impl SchemeDataView {
    /// The view of the scheme `scheme` of the project `data`;
    /// `findings` are those of the whole project. `None` when the
    /// project has no such scheme.
    pub fn new(
        data: ProjectData,
        scheme: i64,
        findings: Vec<Finding>,
        documents: Vec<DocumentView>,
        can_undo: bool,
    ) -> Option<SchemeDataView> {
        let shown = data.schemes.iter().find(|candidate| candidate.id == scheme)?;
        let summary = summary(&data, &findings, shown);
        let schemes = summaries(&data, &findings);

        let tile_of = |id: i64| data.tiles.iter().find(|tile| tile.id == id);
        let in_scheme = |id: i64| tile_of(id).is_some_and(|tile| tile.scheme == scheme);

        let links: Vec<LinkView> = data
            .links
            .iter()
            .filter(|link| in_scheme(link.from) || in_scheme(link.to))
            .map(|link| LinkView {
                id: link.id,
                from: link.from,
                to: link.to,
                meaning: tile_of(link.from)
                    .zip(tile_of(link.to))
                    .and_then(|(from, to)| LinkMeaning::of(from.fields.kind, to.fields.kind))
                    .map(LinkMeaning::name)
                    .unwrap_or("needed_by"),
            })
            .collect();

        let mut shadows: Vec<ShadowView> = Vec::new();

        for link in &links {
            for end in [link.from, link.to] {
                let Some(tile) = tile_of(end).filter(|tile| tile.scheme != scheme) else {
                    continue;
                };

                if shadows.iter().any(|shadow| shadow.id == tile.id) {
                    continue;
                }

                let of = data.schemes.iter().find(|candidate| candidate.id == tile.scheme);

                shadows.push(ShadowView {
                    id: tile.id,
                    scheme: tile.scheme,
                    scheme_number: of.map(|of| of.fields.number.clone()).unwrap_or_default(),
                    scheme_name: of.map(|of| of.fields.name.clone()).unwrap_or_default(),
                    kind: tile.fields.kind.name(),
                    identifier: tile.fields.identifier.clone(),
                    title: tile.fields.title.clone(),
                });
            }
        }

        let tiles: Vec<Tile> = data.tiles.iter().filter(|tile| tile.scheme == scheme).cloned().collect();
        let findings = findings
            .into_iter()
            .filter(|finding| tiles.iter().any(|tile| tile.id == finding.tile))
            .map(FindingView::from)
            .collect();

        Some(SchemeDataView {
            project: data.project.into(),
            scheme: summary,
            schemes,
            tiles: tiles.into_iter().map(TileView::from).collect(),
            shadows,
            links,
            findings,
            documents,
            sections: OutputSection::ALL
                .into_iter()
                .map(|section| SectionView {
                    name: section.name(),
                    title: section.title(),
                    description: section.description(),
                })
                .collect(),
            can_undo,
        })
    }
}

/// A range of pages of one document, for an analysis.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageRange {
    pub document_id: i64,
    pub first_page: u32,
    pub last_page: u32,
}

/// An event of an analysis.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum AnalysisEvent {
    /// A scheme starts, in an analysis of several (counted from 1).
    Scheme { index: usize, count: usize, number: String, name: String },
    /// A part starts (counted from 1).
    Part { index: usize, count: usize, document: String, first_page: u32, last_page: u32 },
    /// The model is answering: the characters written so far.
    Writing { characters: usize },
    /// What the part gave.
    Applied { added: usize, joined: usize, links: usize, dropped: Vec<String> },
    /// What a table of contents gave: new proposed schemes, chapters
    /// the project has already, chapters whose pages were not found.
    Chapters { added: usize, known: usize, without_pages: usize, dropped: Vec<String> },
    /// The part gave nothing.
    Failed { message: String },
    /// The analysis is over.
    End { stopped: bool },
}
