//! The rules of a test project: the fields of a project, a scheme and
//! a tile, the links (which kinds they join; no cycle), and the errors
//! when a rule is broken. The rules work on the objects in memory; a
//! change that keeps them is written by `db/test_projects/`.

use std::fmt;

use crate::model::standards::MAX_QUOTE;
use crate::model::text::{Invalid, check_text};

use super::{Link, MAX_SHORT, MAX_SOURCES, MAX_TEXT, MAX_TITLE, ProjectFields, SchemeFields, TileFields, TileKind};

/// What a link means, from the kinds it joins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkMeaning {
    NeededBy,
    ReportedIn,
    Provides,
    /// A requirement that must be met before another one.
    Precedes,
}

impl LinkMeaning {
    /// The meaning of a link from `from` to `to`; `None` when such a
    /// link is not allowed.
    pub fn of(from: TileKind, to: TileKind) -> Option<LinkMeaning> {
        match (from, to) {
            (TileKind::Input, TileKind::Requirement) => Some(LinkMeaning::NeededBy),
            (TileKind::Requirement, TileKind::Output) => Some(LinkMeaning::ReportedIn),
            (TileKind::Requirement, TileKind::Input) => Some(LinkMeaning::Provides),
            (TileKind::Requirement, TileKind::Requirement) => Some(LinkMeaning::Precedes),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            LinkMeaning::NeededBy => "needed_by",
            LinkMeaning::ReportedIn => "reported_in",
            LinkMeaning::Provides => "provides",
            LinkMeaning::Precedes => "precedes",
        }
    }
}

/// Why a scheme, a tile or a link was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemeError {
    /// A field breaks a rule; holds the field and the reason.
    Invalid(&'static str, String),
    /// Another project has this name.
    NameTaken,
    /// The project, scheme, tile, link or document does not exist.
    NotFound(&'static str),
    /// A link between kinds that cannot be joined.
    LinkNotAllowed(TileKind, TileKind),
    /// The two tiles are joined already.
    LinkExists,
    /// The link would close a cycle.
    Cycle,
}

impl fmt::Display for SchemeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SchemeError::Invalid(_, reason) => formatter.write_str(reason),
            SchemeError::NameTaken => formatter.write_str("another project has this name"),
            SchemeError::NotFound(what) => write!(formatter, "the {what} does not exist"),
            SchemeError::LinkNotAllowed(from, to) => write!(
                formatter,
                "{} cannot be linked to {}: inputs go to requirements, requirements to outputs, to the inputs \
                 they provide or to the requirements they precede",
                from.with_article(),
                to.with_article()
            ),
            SchemeError::LinkExists => formatter.write_str("the two tiles are linked already"),
            SchemeError::Cycle => formatter.write_str("the link would close a circle: a requirement would need itself"),
        }
    }
}

impl std::error::Error for SchemeError {}

impl From<Invalid> for SchemeError {
    fn from(error: Invalid) -> SchemeError {
        SchemeError::Invalid(error.field, error.reason)
    }
}

impl ProjectFields {
    /// Checks the fields (the documents are checked by the storage).
    pub fn check(&self) -> Result<(), SchemeError> {
        if self.name.trim().is_empty() {
            return Err(SchemeError::Invalid("name", "the project needs a name".to_owned()));
        }

        check_text("name", &self.name, MAX_TITLE, false)?;
        check_text("version", &self.version, MAX_SHORT, false)?;
        check_text("author", &self.author, MAX_SHORT, false)?;
        Ok(check_text("description", &self.description, MAX_TEXT, true)?)
    }
}

impl SchemeFields {
    /// Checks the fields (whether the document of the pages belongs
    /// to the project is checked by the storage).
    pub fn check(&self) -> Result<(), SchemeError> {
        if self.name.trim().is_empty() {
            return Err(SchemeError::Invalid("name", "the scheme needs a name".to_owned()));
        }

        check_text("name", &self.name, MAX_TITLE, false)?;
        check_text("number", &self.number, MAX_SHORT, false)?;

        if let Some(pages) = self.pages
            && (pages.first_page == 0 || pages.last_page < pages.first_page)
        {
            return Err(SchemeError::Invalid(
                "pages",
                "the pages of a scheme run from its first page (counted from 1) to its last".to_owned(),
            ));
        }

        Ok(())
    }
}

impl TileFields {
    /// Checks the fields of a tile by the rules of its kind. The pages
    /// of the sources are checked by the storage, which knows the
    /// documents.
    pub fn check(&self) -> Result<(), SchemeError> {
        if self.title.trim().is_empty() {
            return Err(SchemeError::Invalid("title", "the tile needs a title".to_owned()));
        }

        check_text("title", &self.title, MAX_TITLE, false)?;
        check_text("identifier", &self.identifier, MAX_SHORT, false)?;
        check_text("place", &self.place, MAX_SHORT, false)?;
        check_text("description", &self.description, MAX_TEXT, true)?;
        check_text("instructions", &self.instructions, MAX_TEXT, true)?;

        if self.sources.len() > MAX_SOURCES {
            return Err(SchemeError::Invalid("sources", format!("a tile has at most {MAX_SOURCES} sources")));
        }

        for source in &self.sources {
            if source.quote.trim().is_empty() {
                return Err(SchemeError::Invalid("sources", "a source needs the quoted text".to_owned()));
            }

            check_text("quote", &source.quote, MAX_QUOTE, true)?;

            if source.page == 0 {
                return Err(SchemeError::Invalid("sources", "pages are counted from 1".to_owned()));
            }
        }

        if let Some(external) = &self.external {
            if self.kind != TileKind::Input {
                return Err(SchemeError::Invalid(
                    "external",
                    "only an input can be described in another document".to_owned(),
                ));
            }

            if external.document.trim().is_empty() {
                return Err(SchemeError::Invalid("external", "name the document the input is described in".to_owned()));
            }

            check_text("document", &external.document, MAX_SHORT, false)?;
            check_text("place", &external.place, MAX_SHORT, false)?;
        }

        if self.kind != TileKind::Output && (!self.sections.is_empty() || !self.instructions.trim().is_empty()) {
            return Err(SchemeError::Invalid(
                "sections",
                "only an output has sections of a report and instructions".to_owned(),
            ));
        }

        if self.kind == TileKind::Output && self.sections.is_empty() {
            return Err(SchemeError::Invalid("sections", "an output needs at least one section".to_owned()));
        }

        if !self.x.is_finite() || !self.y.is_finite() {
            return Err(SchemeError::Invalid("position", "the position is not a number".to_owned()));
        }

        Ok(())
    }
}

/// Checks a new link from `from` to `to` against the tiles (id and
/// kind) and the links of its scheme.
pub fn check_link(tiles: &[(i64, TileKind)], links: &[Link], from: i64, to: i64) -> Result<LinkMeaning, SchemeError> {
    let kind = |id: i64| tiles.iter().find(|(tile, _)| *tile == id).map(|(_, kind)| *kind);

    let (Some(from_kind), Some(to_kind)) = (kind(from), kind(to)) else {
        return Err(SchemeError::NotFound("tile"));
    };

    let meaning = LinkMeaning::of(from_kind, to_kind).ok_or(SchemeError::LinkNotAllowed(from_kind, to_kind))?;

    if links.iter().any(|link| (link.from, link.to) == (from, to)) {
        return Err(SchemeError::LinkExists);
    }

    if reaches(links, to, from) {
        return Err(SchemeError::Cycle);
    }

    Ok(meaning)
}

/// Whether `target` can be reached from `start` along the links.
fn reaches(links: &[Link], start: i64, target: i64) -> bool {
    let mut seen = vec![start];
    let mut next = vec![start];

    while let Some(tile) = next.pop() {
        if tile == target {
            return true;
        }

        for link in links.iter().filter(|link| link.from == tile) {
            if !seen.contains(&link.to) {
                seen.push(link.to);
                next.push(link.to);
            }
        }
    }

    false
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "rules_tests.rs"]
mod tests;
