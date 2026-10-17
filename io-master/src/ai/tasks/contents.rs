//! The task `contents` of the AI (`ai/api/tasks.rs`, role *text
//! analysis*): the question with the table of contents of a standard,
//! and the reading of the answer, the chapters (step 27 of
//! `docs/todo.md`). Where the chapters are and which schemes they
//! become is found by `test_projects/analysis/contents.rs`.

use serde_json::Value;

use crate::model::analysis::{Part, ProposedChapter};
use crate::model::standards::Document;

use super::json::json_object;

/// The instructions to the model when the user has not changed them
/// (the task `contents` of `ai/api/tasks.rs`; changed ones are kept in
/// `db/ai/instructions/`).
pub const DEFAULT_INSTRUCTIONS: &str = r#"You help to map a standard for an evaluation. The standard will be split into chapters; each chapter becomes a scheme of its own, in which its requirements, their inputs and outputs are drawn later.

Below is the table of contents of the standard. Propose the chapters that hold requirements (criteria, activities, tests) of the product or of the evaluation. Rules:
1. Choose the level of the table at which a chapter holds a coherent group of requirements, usually the second level (for example "9.1", not "9" and not "9.1.5"). Leave out the introduction, the scope, the references, the terms, the annexes without requirements and the bibliography.
2. Copy the number and the title of each chapter exactly as the table gives them.
3. Give the page number the table shows for each chapter, or null when it shows none.
4. Keep the order of the table.

Answer with one JSON object and nothing else, in this form:
{"chapters": [{"number": "9.1", "title": "...", "page": 41}]}"#;

/// The message to the model for the table of contents in `part` of
/// `document`.
pub fn prompt(instructions: &str, document: &Document, part: &Part) -> String {
    format!(
        "{}\n\nThe table of contents of {}, pages {} to {}.\n\n{}",
        instructions.trim(),
        document.fields.title,
        part.first_page,
        part.last_page,
        part.text
    )
}

/// Reads the answer of the model. A chapter without a title is
/// dropped; the rest is kept in the order of the answer.
pub fn read(answer: &str) -> Result<(Vec<ProposedChapter>, Vec<String>), String> {
    let value = json_object(answer).ok_or("the answer holds no JSON object")?;
    let text = |item: &Value, name: &str| item.get(name).and_then(Value::as_str).unwrap_or_default().trim().to_owned();

    let mut chapters = Vec::new();
    let mut dropped = Vec::new();

    for (index, item) in value.get("chapters").and_then(Value::as_array).into_iter().flatten().enumerate() {
        let title = text(item, "title");

        if title.is_empty() {
            dropped.push(format!("chapter {}: no title", index + 1));
            continue;
        }

        chapters.push(ProposedChapter {
            number: text(item, "number"),
            title,
            printed_page: item.get("page").and_then(Value::as_u64).map(|page| page as u32).filter(|page| *page > 0),
        });
    }

    Ok((chapters, dropped))
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "contents_tests.rs"]
mod tests;
