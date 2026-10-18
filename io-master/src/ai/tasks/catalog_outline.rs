//! The task `catalog_outline` of the AI (role *text analysis*): the
//! question with the table of contents (or the first pages) of a
//! document of a standard, and the reading of the answer: the groups of
//! an OSCAL catalog (classes and families, sections, activities),
//! proposed with the passages they come from. Applying them to a
//! catalog is the work of the business (`business/oscal/building.rs`);
//! the evaluator accepts or rejects every one.
//!
//! The program knows no standard: what a group is in a given standard
//! is said by the instructions, which the evaluator may change
//! (`db/ai/instructions/`).

use serde_json::Value;

use crate::model::oscal::catalog::Group;
use crate::model::oscal::common::Property;
use crate::model::oscal::values::token;
use crate::model::standards::{Document, MAX_QUOTE, Part};

use super::json::{json_object, text_of};

/// The instructions when the evaluator has not changed them.
pub const DEFAULT_INSTRUCTIONS: &str = r#"You help to represent a document of a standard in NIST OSCAL, as a catalog. A catalog holds groups (for example the classes and families of Common Criteria, the sections of ETSI EN 303 645, the activities of the CEM), and the groups hold the requirements (controls), which are read later, group by group.

Below is the table of contents (or the first pages) of the document. Propose its groups. Rules:
1. Use the structure of the document: a group for each part that holds requirements; a group inside another one where the document nests them (a family inside its class).
2. Copy the identifier ("ADV", "ADV_FSP", "5.1") and the title of each group exactly as the document gives them.
3. Quote the exact text of the heading of each group, with its page number, as it appears in the text (copy it, do not paraphrase). A quote that is not in the text will be marked as not found.
4. Leave out the introduction, the scope, the references, the terms and the annexes without requirements.

Answer with one JSON object and nothing else, in this form:
{"groups": [{"identifier": "ADV", "title": "Development", "parent": null, "quote": {"page": 12, "text": "..."}},
            {"identifier": "ADV_FSP", "title": "Functional specification", "parent": "ADV", "quote": {"page": 14, "text": "..."}}]}"#;

/// A group as the AI proposed it.
#[derive(Debug, Clone, PartialEq)]
pub struct ProposedGroup {
    /// The group of OSCAL (without content), its id made from the
    /// identifier, the identifier kept as its label.
    pub group: Group,
    /// The id of the group it goes into; `None`: the top of the catalog.
    pub parent: Option<String>,
    /// The passage of its heading: page and text, not checked yet.
    pub quote: Option<(u32, String)>,
}

/// The answer, read.
#[derive(Debug, Clone, PartialEq)]
pub struct Outline {
    pub groups: Vec<ProposedGroup>,
    /// What could not be read, and why.
    pub dropped: Vec<String>,
}

/// The name of a document of a standard, for a question.
pub(super) fn document_name(document: &Document) -> String {
    let fields = &document.fields;
    let mut name = fields.title.clone();

    if !fields.part.is_empty() {
        name.push_str(&format!(", {}", fields.part));
    }

    if !fields.version.is_empty() {
        name.push_str(&format!(" ({})", fields.version));
    }

    name
}

/// The message to the model: the instructions, the groups the catalog
/// has already, and the text of `part`.
pub fn prompt(instructions: &str, document: &Document, part: &Part, known: &[(String, String)]) -> String {
    let mut text = format!("{}\n\n", instructions.trim());

    if !known.is_empty() {
        text.push_str("Groups the catalog has already (do not repeat them; use their identifier as a parent):\n");

        for (id, title) in known {
            text.push_str(&format!("- {id}: {title}\n"));
        }

        text.push('\n');
    }

    text.push_str(&format!(
        "The document: {}, pages {} to {}.\n\n{}",
        document_name(document),
        part.first_page,
        part.last_page,
        part.text
    ));

    text
}

/// The label of an object: its identifier as the standard writes it.
pub(super) fn label(identifier: &str) -> Property {
    Property {
        name: "label".to_owned(),
        value: identifier.to_owned(),
        ns: None,
        class: None,
        other: Default::default(),
    }
}

/// A quote of `item` within `part`; a page outside the part drops it.
pub(super) fn quote_of(item: &Value, part: &Part) -> Option<(u32, String)> {
    let page = u32::try_from(item.get("page")?.as_u64()?).ok()?;
    let text: String = text_of(item, "text").chars().take(MAX_QUOTE).collect();

    (!text.is_empty() && page >= part.first_page && page <= part.last_page).then_some((page, text))
}

/// Reads the answer of the model for `part`. A group that cannot be
/// read is dropped with the reason; the rest is kept in its order.
pub fn read(answer: &str, part: &Part) -> Result<Outline, String> {
    let value = json_object(answer).ok_or("the answer holds no JSON object")?;
    let mut outline = Outline { groups: Vec::new(), dropped: Vec::new() };

    for (index, item) in value.get("groups").and_then(Value::as_array).into_iter().flatten().enumerate() {
        let identifier = text_of(item, "identifier");
        let title = text_of(item, "title");

        if title.is_empty() || title.contains('\n') {
            outline.dropped.push(format!("group {}: no title of one line", index + 1));
            continue;
        }

        let id = token(if identifier.is_empty() { &title } else { &identifier });
        let parent = Some(text_of(item, "parent")).filter(|parent| !parent.is_empty()).map(|parent| token(&parent));

        outline.groups.push(ProposedGroup {
            group: Group {
                id: Some(id),
                class: None,
                title,
                params: Vec::new(),
                props: if identifier.is_empty() { Vec::new() } else { vec![label(&identifier)] },
                links: Vec::new(),
                parts: Vec::new(),
                groups: Vec::new(),
                controls: Vec::new(),
                other: Default::default(),
            },
            parent,
            quote: item.get("quote").and_then(|quote| quote_of(quote, part)),
        });
    }

    Ok(outline)
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "catalog_outline_tests.rs"]
mod tests;
