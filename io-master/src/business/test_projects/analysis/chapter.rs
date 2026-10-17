//! The analysis of a standard with AI: the model reads the pages of
//! one scheme (a chapter of the standard) part by part and proposes
//! tiles and links (steps 20 and 27 of `docs/todo.md`).
//!
//! The task `chapter` of the AI (`ai/api/tasks.rs`), asked in the role
//! *text analysis*. Its pipeline, for every part of the pages
//! (`parts.rs`):
//!
//! ```text
//! question()   from the open project in memory: the instructions, the
//!              chapter, the tiles known so far (of this scheme, and the
//!              requirements of the others), the text of one part; the
//!              text itself is made by ai/tasks/chapter.rs (prompt)
//!   ai/api/completions.rs   the model answers over the connection of the role
//! answer()     ai/tasks/chapter.rs reads the answer (read): one JSON
//!              object with "tiles" and "links";
//!              apply() every proposal checked and stored as a *proposed*
//!              tile in the open project (written through by db/)
//! ```
//!
//! **Nothing the model says is taken on trust.** Every tile is checked
//! by the rules of `model/test_projects/rules.rs`, every quote is looked for on its page
//! (`quote.rs`) and kept with `found` set accordingly, links of the
//! wrong kinds or that close a circle are dropped, and everything
//! enters the scheme as *proposed*: the user accepts, corrects or
//! rejects it (P1 of `architecture.md`, "the evaluator decides").
//!
//! A requirement found twice (in two parts, or again in a later run)
//! is one tile: a proposal with the same kind and identifier, or the
//! same title, as a tile of the scheme is joined to it; links of the
//! proposal then go to that tile. A requirement is looked for in the
//! whole project, so that a chapter that names a requirement of
//! another chapter links to it instead of copying it; inputs and
//! outputs only in their own scheme.

use crate::business::standards;
use crate::business::standards::quote::{is_on_page, normalize};
use crate::business::test_projects::error::StoreError;
use crate::business::test_projects::schemes::existing;
use crate::business::test_projects::workspace::OpenProject;
use crate::business::test_projects::{links, tiles as tile_crud};
use crate::db::user::database::UserDb;
use crate::model::test_projects::rules::ProjectError;
use crate::model::test_projects::{ProposalState, Tile, TileFields, TileKind};
use crate::model::text::Entity;

use crate::ai::api::completions::{self};
use crate::model::ai::Turn;

use crate::ai::tasks::chapter::{prompt, read};
use crate::model::analysis::{ChapterProposal, Part};

/// What a part gave.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Applied {
    /// New proposed tiles.
    pub added: usize,
    /// Proposals joined to a tile of the scheme.
    pub joined: usize,
    /// New links.
    pub links: usize,
    /// What was dropped, and why.
    pub dropped: Vec<String>,
}

/// The tile of the scheme a proposal is the same as: the same kind
/// and identifier, or (without identifiers) the same title.
fn same(tiles: &[Tile], fields: &TileFields) -> Option<i64> {
    let identifier = normalize(&fields.identifier);
    let title = normalize(&fields.title);

    tiles
        .iter()
        .find(|tile| {
            tile.fields.kind == fields.kind
                && if identifier.is_empty() {
                    normalize(&tile.fields.title) == title
                } else {
                    normalize(&tile.fields.identifier) == identifier
                }
        })
        .map(|tile| tile.id)
}

/// The column of a kind on the canvas: inputs left, requirements in
/// the middle, outputs right.
fn column(kind: TileKind) -> f64 {
    match kind {
        TileKind::Input => 0.0,
        TileKind::Requirement => 420.0,
        TileKind::Output => 840.0,
    }
}

/// The distance of two tiles in one column.
const ROW: f64 = 140.0;

/// Stores a proposal in a scheme: the new tiles as proposed, below
/// the tiles there are, with their quotes checked; a proposal the same
/// as a tile of the scheme (or a requirement of the project) joined to
/// it (its new quotes added while that tile is still proposed); then
/// the links, also to tiles of other schemes of the project.
pub fn apply(
    db: &UserDb,
    open: &mut OpenProject,
    scheme: i64,
    proposal: &ChapterProposal,
) -> Result<Applied, StoreError> {
    let mut applied = Applied { dropped: proposal.dropped.clone(), ..Applied::default() };

    existing(open, scheme)?;
    // Every tile of the project: the tiles of this scheme, and the
    // requirements of the others, may be joined; links may reach all.
    let mut tiles = open.data().tiles.clone();
    // The key of every proposal and the tile it became.
    let mut keys: Vec<(String, i64)> = Vec::new();

    for proposed in &proposal.tiles {
        let mut fields = proposed.fields.clone();

        for source in &mut fields.sources {
            source.found = standards::documents::page(db, source.document_id, source.page)?
                .is_some_and(|page| is_on_page(&source.quote, &page));
        }

        let candidates: Vec<Tile> = tiles
            .iter()
            .filter(|tile| tile.scheme == scheme || tile.fields.kind == TileKind::Requirement)
            .cloned()
            .collect();

        if let Some(existing) = same(&candidates, &fields) {
            let tile = tiles.iter_mut().find(|tile| tile.id == existing).expect("found above");

            if tile.fields.state == ProposalState::Proposed {
                let mut sources = tile.fields.sources.clone();

                for source in fields.sources {
                    if !sources.iter().any(|known| known.page == source.page && known.quote == source.quote) {
                        sources.push(source);
                    }
                }

                let mut changed = tile.fields.clone();
                changed.sources = sources;

                if tile_crud::update(db, open, existing, &changed).is_ok() {
                    tile.fields = changed;
                }
            }

            keys.push((proposed.key.clone(), existing));
            applied.joined += 1;
            continue;
        }

        let lowest = tiles
            .iter()
            .filter(|tile| tile.scheme == scheme && tile.fields.kind == fields.kind)
            .map(|tile| tile.fields.y)
            .fold(None, |lowest: Option<f64>, y| Some(lowest.map_or(y, |lowest| lowest.max(y))));

        fields.x = column(fields.kind);
        fields.y = lowest.map_or(0.0, |y| y + ROW);

        match tile_crud::create(db, open, scheme, &fields) {
            Ok(id) => {
                keys.push((proposed.key.clone(), id));
                tiles.push(Tile { id, scheme, fields });
                applied.added += 1;
            }
            Err(StoreError::Rule(error)) => applied.dropped.push(format!("{}: {error}", proposed.key)),
            Err(error) => return Err(error),
        }
    }

    // An end of a link: a key of this answer, or the identifier of a
    // tile of the project (of this scheme first).
    let end = |name: &str| {
        keys.iter().find(|(key, _)| key == name).map(|(_, id)| *id).or_else(|| {
            let wanted = normalize(name);
            let named = |tile: &&Tile| !wanted.is_empty() && normalize(&tile.fields.identifier) == wanted;

            tiles
                .iter()
                .filter(|tile| tile.scheme == scheme)
                .find(named)
                .or_else(|| tiles.iter().find(named))
                .map(|tile| tile.id)
        })
    };

    for (from, to) in &proposal.links {
        let (Some(from_id), Some(to_id)) = (end(from), end(to)) else {
            applied.dropped.push(format!("the link {from} → {to}: an end is unknown"));
            continue;
        };

        match links::create(db, open, from_id, to_id) {
            Ok(_) => applied.links += 1,
            // The same link proposed twice (or already drawn): what
            // was asked for is there, so nothing is told.
            Err(StoreError::Rule(ProjectError::LinkExists)) => {}
            Err(StoreError::Rule(error)) => applied.dropped.push(format!("the link {from} → {to}: {error}")),
            Err(error) => return Err(error),
        }
    }

    Ok(applied)
}

/// What the answer of the model to one part came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answered {
    /// The proposals were applied to the scheme.
    Applied(Applied),
    /// The answer could not be read; the reason, for the user.
    Unreadable(String),
}

/// The question for `part` of the pages of `scheme`, made from the
/// open project as it is now (the tiles found in earlier parts are
/// known to the model) and the instructions of the task.
pub fn question(
    db: &UserDb,
    open: &OpenProject,
    scheme: i64,
    part: &Part,
    instructions: &str,
) -> Result<Vec<Turn>, StoreError> {
    let stored = existing(open, scheme)?;
    let document = standards::documents::get(db, part.document_id)?
        .ok_or(StoreError::Rule(ProjectError::NotFound(Entity::Document)))?;
    let (own, others): (Vec<Tile>, Vec<Tile>) =
        open.data().tiles.iter().cloned().partition(|tile| tile.scheme == scheme);

    Ok(completions::single(prompt(instructions, &document, part, stored, &own, &others)))
}

/// Reads the answer of the model to `part` and applies it to `scheme`
/// of the open project.
pub fn answer(
    db: &UserDb,
    open: &mut OpenProject,
    scheme: i64,
    part: &Part,
    text: &str,
) -> Result<Answered, StoreError> {
    match read(text, part) {
        Ok(proposal) => Ok(Answered::Applied(apply(db, open, scheme, &proposal)?)),
        Err(reason) => Ok(Answered::Unreadable(reason)),
    }
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "chapter_tests.rs"]
mod tests;
