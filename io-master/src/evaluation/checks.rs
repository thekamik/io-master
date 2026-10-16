//! The checks of a scheme: the gaps that keep it from being a
//! complete plan of an evaluation (step 21 of `docs/todo.md`).
//!
//! The rules of `scheme.rs` refuse what can never be right (a link of
//! the wrong kinds, a circle). The checks here find what is not done
//! yet; a scheme with findings can still be stored, and the user marks
//! it ready only when they accept what is left.
//!
//! ```text
//! requirement without inputs     nothing is asked before it is met
//!                                (no input, no preceding requirement)
//! input needed by nothing        an input no requirement uses
//! requirement in no output       its result would not be reported
//! output without requirements    a report of nothing
//! input to be completed          described in a document not loaded
//! proposed tile                  proposed by the AI, not accepted
//! quote not found                a source that is not on its page
//! requirement without a source   not traced to the standard
//! ```

use super::scheme::{Link, LinkMeaning, Tile, TileKind, ProposalState};

/// The kind of a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingKind {
    RequirementWithoutInputs,
    UnusedInput,
    RequirementNotReported,
    EmptyOutput,
    InputToComplete,
    Proposed,
    QuoteNotFound,
    RequirementWithoutSource,
}

impl FindingKind {
    /// The name the page knows.
    pub fn name(self) -> &'static str {
        match self {
            FindingKind::RequirementWithoutInputs => "requirement_without_inputs",
            FindingKind::UnusedInput => "unused_input",
            FindingKind::RequirementNotReported => "requirement_not_reported",
            FindingKind::EmptyOutput => "empty_output",
            FindingKind::InputToComplete => "input_to_complete",
            FindingKind::Proposed => "proposed",
            FindingKind::QuoteNotFound => "quote_not_found",
            FindingKind::RequirementWithoutSource => "requirement_without_source",
        }
    }

    /// The sentence for the user, for one tile.
    pub fn sentence(self) -> &'static str {
        match self {
            FindingKind::RequirementWithoutInputs => "The requirement needs no input, so it is met at once.",
            FindingKind::UnusedInput => "No requirement needs this input.",
            FindingKind::RequirementNotReported => "No output reports this requirement.",
            FindingKind::EmptyOutput => "The output reports no requirement.",
            FindingKind::InputToComplete => "The input is described in a document that is not loaded.",
            FindingKind::Proposed => "Proposed by the AI and not accepted yet.",
            FindingKind::QuoteNotFound => "A quoted source is not on its page.",
            FindingKind::RequirementWithoutSource => "The requirement is not traced to a passage of the standard.",
        }
    }
}

/// One finding: its kind and the tile it is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Finding {
    pub kind: FindingKind,
    pub tile: i64,
}

/// The findings of a scheme, in the order of its tiles.
pub fn check(tiles: &[Tile], links: &[Link]) -> Vec<Finding> {
    let kind_of = |id: i64| tiles.iter().find(|tile| tile.id == id).map(|tile| tile.fields.kind);

    // The meaning of every link, from the kinds it joins.
    let meanings: Vec<(i64, i64, LinkMeaning)> = links
        .iter()
        .filter_map(|link| {
            let meaning = LinkMeaning::of(kind_of(link.from)?, kind_of(link.to)?)?;
            Some((link.from, link.to, meaning))
        })
        .collect();

    let has = |from: Option<i64>, to: Option<i64>, wanted: LinkMeaning| {
        meanings.iter().any(|(link_from, link_to, meaning)| {
            *meaning == wanted && from.is_none_or(|id| id == *link_from) && to.is_none_or(|id| id == *link_to)
        })
    };

    let mut findings = Vec::new();

    for tile in tiles {
        let id = tile.id;
        let fields = &tile.fields;
        let mut add = |kind| findings.push(Finding { kind, tile: id });

        if fields.state == ProposalState::Proposed {
            add(FindingKind::Proposed);
        }

        match fields.kind {
            TileKind::Requirement => {
                // An input, or a requirement that must be met first,
                // is something asked before this one is met.
                if !has(None, Some(id), LinkMeaning::NeededBy) && !has(None, Some(id), LinkMeaning::Precedes) {
                    add(FindingKind::RequirementWithoutInputs);
                }

                if !has(Some(id), None, LinkMeaning::ReportedIn) {
                    add(FindingKind::RequirementNotReported);
                }

                if fields.sources.is_empty() {
                    add(FindingKind::RequirementWithoutSource);
                }
            }
            TileKind::Input => {
                if !has(Some(id), None, LinkMeaning::NeededBy) {
                    add(FindingKind::UnusedInput);
                }

                if fields.external.is_some() {
                    add(FindingKind::InputToComplete);
                }
            }
            TileKind::Output => {
                if !has(None, Some(id), LinkMeaning::ReportedIn) {
                    add(FindingKind::EmptyOutput);
                }
            }
        }

        if fields.sources.iter().any(|source| !source.found) {
            add(FindingKind::QuoteNotFound);
        }
    }

    findings
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "checks_tests.rs"]
mod tests;
