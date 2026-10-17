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
//!              requirements of the others), the text of one part
//!   ai/api/completions.rs   the model answers over the connection of the role
//! answer()     read() the answer: one JSON object with "tiles" and "links"
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

use serde_json::Value;

use crate::db::user::database::UserDb;
use crate::model::standards::Document;
use crate::model::standards::MAX_QUOTE;
use crate::model::test_projects::rules::SchemeError;
use crate::model::test_projects::{External, OutputSection, ProposalState, Scheme, Source, Tile, TileFields, TileKind};
use crate::standards;
use crate::standards::quote::{is_on_page, normalize};
use crate::test_projects::error::StoreError;
use crate::test_projects::schemes::read::existing;
use crate::test_projects::workspace::OpenProject;
use crate::test_projects::{links, tiles as tile_crud};

use crate::ai::api::completions::{self};
use crate::model::ai::Turn;

use super::parts::Part;

/// The instructions to the model when the user has not changed them
/// (the task `chapter` of `ai/api/tasks.rs`; changed ones are kept in
/// `db/ai/instructions/`).
pub const DEFAULT_INSTRUCTIONS: &str = r#"You help to build an evaluation scheme from a standard. A scheme is the plan of an evaluation (the plan of the tests), drawn as tiles of three kinds:

- "input": what the evaluation needs: documentation of the manufacturer, a sample of the product, access, the result of another requirement, or an element described in another document of the standard.
- "requirement": what the standard imposes on the product or on the evaluator: a criterion, an activity, a test. It is met when all its inputs are met.
- "output": the result in the standard form of a report; it reports one or more requirements.

Links join tiles: from an input to the requirement that needs it; from a requirement to the output that reports it; from a requirement to an input it provides (a product of it, such as a test report, that another requirement needs); from a requirement to a requirement it precedes (the first must be met before the second).

Read the part of the standard below and propose the tiles and links it describes. Rules:
1. Quote the exact text of the standard each tile is based on, with its page number, as it appears in the text (copy it, do not paraphrase). A quote that is not in the text will be rejected.
2. Use the identifiers of the standard (for example "ADV_FSP.4.1E" or "5.3.2") in "identifier", and the clause or table in "place".
3. When a requirement needs something described in another document of the standard that is not given here, propose an input with "external": {"document": the name of that document, "place": where in it}.
4. Do not repeat tiles listed as already known; link to them by their identifier in "from" or "to" instead of a key.
5. Propose an output for a group of requirements that are reported together.

Answer with one JSON object and nothing else, in this form:
{"tiles": [{"key": "t1", "kind": "requirement", "identifier": "...", "title": "...", "description": "...", "place": "...", "quotes": [{"page": 12, "text": "..."}], "external": null}],
 "links": [{"from": "t2", "to": "t1"}]}"#;

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

/// One line about a known tile: "- requirement 9.1.5: Title".
fn known_line(tile: &Tile) -> String {
    let fields = &tile.fields;
    let identifier = if fields.identifier.is_empty() { "(no identifier)" } else { &fields.identifier };

    format!("- {} {}: {}\n", fields.kind.name(), identifier, fields.title)
}

/// The message to the model for one part of `scheme`. `own` are the
/// tiles of the scheme known so far; `others` the tiles of the other
/// schemes of the project, of which only the requirements are named
/// (short, so that a large project does not fill the context).
pub fn prompt(
    instructions: &str,
    document: &Document,
    part: &Part,
    scheme: &Scheme,
    own: &[Tile],
    others: &[Tile],
) -> String {
    let mut text = String::new();

    text.push_str(instructions.trim());
    text.push_str("\n\n");

    let chapter = format!("{} {}", scheme.fields.number, scheme.fields.name);
    text.push_str(&format!("The chapter analysed: {}. Propose the tiles of this chapter.\n\n", chapter.trim()));

    if !own.is_empty() {
        text.push_str("Tiles of this chapter already known (do not repeat them; link to them by identifier):\n");

        for tile in own {
            text.push_str(&known_line(tile));
        }

        text.push('\n');
    }

    let requirements: Vec<&Tile> = others.iter().filter(|tile| tile.fields.kind == TileKind::Requirement).collect();

    if !requirements.is_empty() {
        text.push_str(
            "Requirements of other chapters (do not repeat them; when this chapter needs one met first, link to it \
             by its identifier):\n",
        );

        for tile in requirements {
            text.push_str(&known_line(tile));
        }

        text.push('\n');
    }

    let mut name = document.fields.title.clone();

    if !document.fields.part.is_empty() {
        name.push_str(", ");
        name.push_str(&document.fields.part);
    }

    if !document.fields.version.is_empty() {
        name.push_str(&format!(" ({})", document.fields.version));
    }

    text.push_str(&format!(
        "The part of the standard: {name}, pages {} to {}.\n\n{}",
        part.first_page, part.last_page, part.text
    ));

    text
}

/// A tile as the model proposed it.
#[derive(Debug, Clone, PartialEq)]
pub struct Proposed {
    pub key: String,
    pub fields: TileFields,
}

/// The answer of the model, read.
#[derive(Debug, Clone, PartialEq)]
pub struct Proposal {
    pub tiles: Vec<Proposed>,
    /// From and to: a key of the answer, or an identifier of a known
    /// tile.
    pub links: Vec<(String, String)>,
    /// What could not be read, and why.
    pub dropped: Vec<String>,
}

/// The JSON object in an answer: the answer itself, or the first
/// object in it (a model may put it in a code block, or add words).
fn json_object(answer: &str) -> Option<Value> {
    if let Ok(value) = serde_json::from_str::<Value>(answer.trim())
        && value.is_object()
    {
        return Some(value);
    }

    let start = answer.find('{')?;
    let end = answer.rfind('}')?;

    serde_json::from_str::<Value>(answer.get(start..=end)?).ok().filter(Value::is_object)
}

fn text_of(value: &Value, name: &str) -> String {
    value.get(name).and_then(Value::as_str).unwrap_or_default().trim().to_owned()
}

/// Reads the answer of the model for `part`. A tile that cannot be
/// read is dropped with the reason; the rest is kept.
pub fn read(answer: &str, part: &Part) -> Result<Proposal, String> {
    let value = json_object(answer).ok_or("the answer holds no JSON object")?;

    let mut proposal = Proposal { tiles: Vec::new(), links: Vec::new(), dropped: Vec::new() };

    for (index, item) in value.get("tiles").and_then(Value::as_array).into_iter().flatten().enumerate() {
        let key = Some(text_of(item, "key")).filter(|key| !key.is_empty()).unwrap_or(format!("tile {}", index + 1));

        let Some(kind) = TileKind::from_name(&text_of(item, "kind")) else {
            proposal.dropped.push(format!("{key}: an unknown kind"));
            continue;
        };

        let mut fields = TileFields::new(kind, &text_of(item, "title"));
        fields.identifier = text_of(item, "identifier");
        fields.description = text_of(item, "description");
        fields.place = text_of(item, "place");
        fields.state = ProposalState::Proposed;

        if kind == TileKind::Output {
            fields.sections = OutputSection::ALL.to_vec();
        }

        if kind == TileKind::Input
            && let Some(external) = item.get("external").filter(|external| external.is_object())
        {
            let document = text_of(external, "document");

            if !document.is_empty() {
                fields.external = Some(External { document, place: text_of(external, "place") });
            }
        }

        for quote in item.get("quotes").and_then(Value::as_array).into_iter().flatten() {
            let page = quote.get("page").and_then(Value::as_u64).unwrap_or(0) as u32;
            let text: String = text_of(quote, "text").chars().take(MAX_QUOTE).collect();

            if text.is_empty() {
                continue;
            }

            if page < part.first_page || page > part.last_page {
                proposal.dropped.push(format!("{key}: a quote of page {page}, outside the part"));
                continue;
            }

            fields.sources.push(Source { document_id: part.document_id, page, quote: text, found: false });
        }

        match fields.check() {
            Ok(()) => proposal.tiles.push(Proposed { key, fields }),
            Err(error) => proposal.dropped.push(format!("{key}: {error}")),
        }
    }

    for item in value.get("links").and_then(Value::as_array).into_iter().flatten() {
        let (from, to) = (text_of(item, "from"), text_of(item, "to"));

        if from.is_empty() || to.is_empty() {
            proposal.dropped.push("a link without its two ends".to_owned());
        } else {
            proposal.links.push((from, to));
        }
    }

    Ok(proposal)
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
pub fn apply(db: &UserDb, open: &mut OpenProject, scheme: i64, proposal: &Proposal) -> Result<Applied, StoreError> {
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
            source.found = standards::read::page(db, source.document_id, source.page)?
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

                if tile_crud::update::tile(db, open, existing, &changed).is_ok() {
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

        match tile_crud::create::tile(db, open, scheme, &fields) {
            Ok(id) => {
                keys.push((proposed.key.clone(), id));
                tiles.push(Tile { id, scheme, fields });
                applied.added += 1;
            }
            Err(StoreError::Scheme(error)) => applied.dropped.push(format!("{}: {error}", proposed.key)),
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

        match links::create::link(db, open, from_id, to_id) {
            Ok(_) => applied.links += 1,
            // The same link proposed twice (or already drawn): what
            // was asked for is there, so nothing is told.
            Err(StoreError::Scheme(SchemeError::LinkExists)) => {}
            Err(StoreError::Scheme(error)) => applied.dropped.push(format!("the link {from} → {to}: {error}")),
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
    let document =
        standards::read::get(db, part.document_id)?.ok_or(StoreError::Scheme(SchemeError::NotFound("document")))?;
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
