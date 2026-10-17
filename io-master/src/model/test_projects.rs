//! A test project: the map of a standard, as schemes of tiles joined
//! by links (steps 16 to 22 and 24 to 27 of `docs/todo.md`).
//!
//! ```text
//! project      the documents of the standard, name, version, author,
//!              "ready"; it holds the schemes and the links
//!  +- scheme   one chapter of the standard: number "9.1", name, the
//!  |           pages it covers; the user sees one scheme at a time
//!  |   +- tile an input, a requirement or an output
//!  +- link     joins two tiles of the project, also of two schemes
//! ```
//!
//! The program knows no standard: a project holds everything, and the
//! same three kinds of tile serve every standard:
//!
//! ```text
//! input        what the evaluation needs (documentation, a sample,
//!              access, the result of another requirement, a part of
//!              another document of the standard)
//! requirement  what the standard imposes; met when all its inputs are
//! output       the result in the standard form of a report (the seven
//!              sections of OutputSection::ALL)
//! ```
//!
//! A **link** joins two tiles of one scheme. Its meaning follows from
//! the kinds it joins, so it has no kind of its own:
//!
//! ```text
//! input       → requirement   the input is needed by the requirement
//! requirement → output        the requirement is reported in the output
//! requirement → input         the requirement provides the input
//!                             (a product of it, such as a test report,
//!                             that another requirement needs)
//! requirement → requirement   the first precedes the second: it must
//!                             be met before the second can be
//! ```
//!
//! Every other pair is refused, and so is a link that closes a cycle
//! (a requirement that would, through other tiles, need itself).
//!
//! ```text
//! model/test_projects.rs         the types (this file)
//! model/model/test_projects/rules.rs   the rules of the fields and the links;
//!                                the error when one is broken
//! test_projects/                 the work on them: the projects open for
//!                                editing, their CRUD, checks, exchange,
//!                                the analysis with the AI
//! ```

/// The longest title of a tile or name of a scheme, in characters.
pub const MAX_TITLE: usize = 300;
/// The longest identifier, place, version or name of a document.
pub const MAX_SHORT: usize = 200;
/// The longest description or instruction, in characters.
pub const MAX_TEXT: usize = 20_000;
/// The most sources a tile may have.
pub const MAX_SOURCES: usize = 50;

/// The kind of a tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TileKind {
    Input,
    Requirement,
    Output,
}

impl TileKind {
    pub const ALL: [TileKind; 3] = [TileKind::Input, TileKind::Requirement, TileKind::Output];

    /// The name kept in the database, in files and by the page.
    pub fn name(self) -> &'static str {
        match self {
            TileKind::Input => "input",
            TileKind::Requirement => "requirement",
            TileKind::Output => "output",
        }
    }

    pub fn from_name(name: &str) -> Option<TileKind> {
        TileKind::ALL.into_iter().find(|kind| kind.name() == name)
    }

    /// The name with its article, for sentences: "an input".
    pub fn with_article(self) -> &'static str {
        match self {
            TileKind::Input => "an input",
            TileKind::Requirement => "a requirement",
            TileKind::Output => "an output",
        }
    }
}

/// Whether the user accepted a tile or a scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalState {
    /// Proposed by the AI (step 20) and not accepted yet.
    Proposed,
    /// Made or accepted by the user.
    Accepted,
}

impl ProposalState {
    pub fn name(self) -> &'static str {
        match self {
            ProposalState::Proposed => "proposed",
            ProposalState::Accepted => "accepted",
        }
    }

    pub fn from_name(name: &str) -> Option<ProposalState> {
        match name {
            "proposed" => Some(ProposalState::Proposed),
            "accepted" => Some(ProposalState::Accepted),
            _ => None,
        }
    }
}

/// A section of the report an output makes (the form agreed on
/// 2026-10-16, `docs/todo.md`, "Evaluation schemes").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OutputSection {
    /// The requirements reported, with the quoted sources.
    Requirements,
    /// What each requirement needed, and what was provided.
    Inputs,
    /// The activities and tests carried out.
    Activities,
    /// The evidence.
    Evidence,
    /// Observations, deviations, open points.
    Findings,
    /// The verdict per requirement, with its rationale.
    Verdict,
    /// The vulnerabilities that may follow from the solution used,
    /// and notes (user, 2026-10-17: one of the four results of every
    /// requirement in the evaluation, step 29 of `docs/todo.md`).
    Vulnerabilities,
}

impl OutputSection {
    /// Every section, in the order of the report.
    pub const ALL: [OutputSection; 7] = [
        OutputSection::Requirements,
        OutputSection::Inputs,
        OutputSection::Activities,
        OutputSection::Evidence,
        OutputSection::Findings,
        OutputSection::Verdict,
        OutputSection::Vulnerabilities,
    ];

    pub fn name(self) -> &'static str {
        match self {
            OutputSection::Requirements => "requirements",
            OutputSection::Inputs => "inputs",
            OutputSection::Activities => "activities",
            OutputSection::Evidence => "evidence",
            OutputSection::Findings => "findings",
            OutputSection::Verdict => "verdict",
            OutputSection::Vulnerabilities => "vulnerabilities",
        }
    }

    pub fn from_name(name: &str) -> Option<OutputSection> {
        OutputSection::ALL.into_iter().find(|section| section.name() == name)
    }

    /// The title of the section in a report.
    pub fn title(self) -> &'static str {
        match self {
            OutputSection::Requirements => "Requirements reported",
            OutputSection::Inputs => "Inputs",
            OutputSection::Activities => "Activities and tests carried out",
            OutputSection::Evidence => "Evidence",
            OutputSection::Findings => "Findings",
            OutputSection::Verdict => "Verdict and rationale",
            OutputSection::Vulnerabilities => "Vulnerabilities and notes",
        }
    }

    /// What the section holds, for the user and for the AI.
    pub fn description(self) -> &'static str {
        match self {
            OutputSection::Requirements => {
                "The identifier, title and place in the standard of each requirement, with the quoted passages."
            }
            OutputSection::Inputs => "What each requirement needed, and what was provided for it.",
            OutputSection::Activities => {
                "The method, the steps, the tools and their configuration, the date, and who carried them out."
            }
            OutputSection::Evidence => "Each piece of evidence: what it is, where it is kept, its checksum.",
            OutputSection::Findings => "Observations, deviations from the requirement, and open points.",
            OutputSection::Verdict => "The verdict of each requirement and its rationale, given by the evaluator.",
            OutputSection::Vulnerabilities => {
                "The vulnerabilities that may follow from the solution used, and further notes."
            }
        }
    }
}

/// A passage of a standard a tile comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    /// The document (`standards/`).
    pub document_id: i64,
    /// The page, counted from 1 as the viewer shows it.
    pub page: u32,
    /// The text copied from the page.
    pub quote: String,
    /// Whether the quote was found on the page (`quote.rs`); set by
    /// the program when the source is stored, never by the user.
    pub found: bool,
}

/// An input described in a document of the standard that is not
/// loaded: to be completed later.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct External {
    /// The document, as the standard names it: "CC Part 3".
    pub document: String,
    /// Where in it: "ALC_FLR.2".
    pub place: String,
}

/// The fields of a tile, as the user or the AI gives them.
#[derive(Debug, Clone, PartialEq)]
pub struct TileFields {
    pub kind: TileKind,
    /// The identifier in the standard, if it has one: "ADV_FSP.4.1E".
    pub identifier: String,
    pub title: String,
    pub description: String,
    /// The place in the standard: a clause, a table.
    pub place: String,
    pub sources: Vec<Source>,
    pub state: ProposalState,
    /// For an input only: described in a document not loaded.
    pub external: Option<External>,
    /// For an output only: the sections of the report that apply.
    pub sections: Vec<OutputSection>,
    /// For an output only: what the laboratory expects written.
    pub instructions: String,
    /// The position on the canvas (step 19).
    pub x: f64,
    pub y: f64,
}

impl TileFields {
    /// A new tile of `kind` with only a title; an output with every
    /// section.
    pub fn new(kind: TileKind, title: &str) -> TileFields {
        TileFields {
            kind,
            identifier: String::new(),
            title: title.to_owned(),
            description: String::new(),
            place: String::new(),
            sources: Vec::new(),
            state: ProposalState::Accepted,
            external: None,
            sections: if kind == TileKind::Output { OutputSection::ALL.to_vec() } else { Vec::new() },
            instructions: String::new(),
            x: 0.0,
            y: 0.0,
        }
    }
}

/// A stored tile.
#[derive(Debug, Clone, PartialEq)]
pub struct Tile {
    pub id: i64,
    /// The scheme the tile is drawn in.
    pub scheme: i64,
    pub fields: TileFields,
}

/// A stored link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Link {
    pub id: i64,
    pub from: i64,
    pub to: i64,
}

/// The fields of a test project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectFields {
    pub name: String,
    pub version: String,
    pub description: String,
    /// Who made it (a head of the laboratory, an evaluator).
    pub author: String,
    /// The documents of the standard it uses.
    pub documents: Vec<i64>,
}

/// The pages of one document a scheme covers: where its chapter is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageSpan {
    pub document_id: i64,
    /// The first and the last page, counted from 1 as the viewer
    /// shows them.
    pub first_page: u32,
    pub last_page: u32,
}

/// The fields of a scheme: one chapter of the standard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemeFields {
    /// The number of the chapter in the standard: "9.1"; may be empty.
    pub number: String,
    pub name: String,
    /// The pages of the chapter, if known: the analysis reads them.
    pub pages: Option<PageSpan>,
    /// Proposed by the AI (from the table of contents) or accepted.
    pub state: ProposalState,
}

/// A stored scheme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scheme {
    pub id: i64,
    /// The project it belongs to.
    pub project: i64,
    pub fields: SchemeFields,
}

/// A stored test project, without its schemes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub id: i64,
    pub fields: ProjectFields,
    /// Marked ready by the user (step 21).
    pub ready: bool,
}

/// A project with everything in it: as it is open in memory
/// (`workspace.rs`) or read from the database.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectData {
    pub project: Project,
    /// In the order of the list of the project.
    pub schemes: Vec<Scheme>,
    /// The tiles of all its schemes.
    pub tiles: Vec<Tile>,
    pub links: Vec<Link>,
}

pub mod rules;
