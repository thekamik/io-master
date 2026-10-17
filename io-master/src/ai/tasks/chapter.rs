//! The task `chapter` of the AI (`ai/api/tasks.rs`, role *text
//! analysis*): the question for one part of the pages of a chapter,
//! and the reading of the answer, one JSON object with "tiles" and
//! "links" (step 20 of `docs/todo.md`). The proposals read here are
//! checked and applied by `test_projects/analysis/chapter.rs`.

use serde_json::Value;

use crate::model::analysis::{ChapterProposal, Part, ProposedTile};
use crate::model::standards::{Document, MAX_QUOTE};
use crate::model::test_projects::{External, OutputSection, ProposalState, Scheme, Source, Tile, TileFields, TileKind};

use super::json::{json_object, text_of};

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

/// Reads the answer of the model for `part`. A tile that cannot be
/// read is dropped with the reason; the rest is kept.
pub fn read(answer: &str, part: &Part) -> Result<ChapterProposal, String> {
    let value = json_object(answer).ok_or("the answer holds no JSON object")?;

    let mut proposal = ChapterProposal { tiles: Vec::new(), links: Vec::new(), dropped: Vec::new() };

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
            Ok(()) => proposal.tiles.push(ProposedTile { key, fields }),
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

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "chapter_tests.rs"]
mod tests;
