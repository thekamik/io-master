//! The task `catalog_contents` of the AI (role *text analysis*, step 50.2
//! of docs/history.md): the question with the first pages of a document of a
//! standard, and the reading of the answer: the pages that hold its table of
//! contents. The evaluator confirms them before the groups are read from
//! them (`catalog_outline.rs`); the program never takes them on trust.

use crate::model_old::standards::{Document, Part};

use super::catalog_outline::document_name;
use super::json::json_object;

/// How many pages from the start of a document the AI reads to find its
/// table of contents: enough for the cover, the foreword and a long
/// table of contents.
pub const FIRST_PAGES: u32 = 25;

/// The instructions when the evaluator has not changed them.
pub const DEFAULT_INSTRUCTIONS: &str = r#"You help to represent a document of a standard in NIST OSCAL, as a catalog. Below are the first pages of the document, each after a line "=== Page N ===".

Find the pages that hold its table of contents (the list of the headings with their page numbers). Give the number N of the first and of the last of these pages, as the lines "=== Page N ===" write them, not the numbers printed in the document.

Answer with one JSON object and nothing else, in this form:
{"first_page": 3, "last_page": 6}
If the pages hold no table of contents, answer {"first_page": null, "last_page": null}."#;

/// The message to the model: the instructions and the text of `part`.
///
/// Called by: `conversion_wizard/contents.rs`.
pub fn prompt(instructions: &str, document: &Document, part: &Part) -> String {
    format!(
        "{}\n\nThe document: {}, pages {} to {}.\n\n{}",
        instructions.trim(),
        document_name(document),
        part.first_page,
        part.last_page,
        part.text
    )
}

/// Reads the answer of the model for `part`: the first and the last page of
/// the table of contents, both within `part`; `None` when the model found
/// none.
///
/// Called by: `conversion_wizard/contents.rs`.
pub fn read(answer: &str, part: &Part) -> Result<Option<(u32, u32)>, String> {
    let value = json_object(answer).ok_or("the answer holds no JSON object")?;
    let first = value.get("first_page");
    let last = value.get("last_page");

    let (Some(first), Some(last)) = (first, last) else {
        return Err("the answer names no pages".to_owned());
    };

    if first.is_null() && last.is_null() {
        return Ok(None);
    }

    let (Some(first), Some(last)) = (first.as_u64(), last.as_u64()) else {
        return Err("the pages of the answer are no numbers".to_owned());
    };

    let (Ok(first), Ok(last)) = (u32::try_from(first), u32::try_from(last)) else {
        return Err("the pages of the answer are too large".to_owned());
    };

    if first > last || first < part.first_page || last > part.last_page {
        return Err(format!("the pages {first} to {last} are not among the pages read"));
    }

    Ok(Some((first, last)))
}

#[cfg(test)]
mod tests {
    //! Unit tests of `catalog_contents.rs`.

    use super::*;

    use crate::model_old::standards::DocumentFields;

    fn part() -> Part {
        Part { document_id: 3, first_page: 1, last_page: 25, text: "=== Page 1 ===\nCover".to_owned() }
    }

    #[test]
    fn the_question_holds_the_instructions_the_document_and_its_pages() {
        let document = Document {
            id: 3,
            fields: DocumentFields {
                title: "Common Criteria".to_owned(),
                part: "Part 3".to_owned(),
                version: String::new(),
            },
            file_name: "cc.pdf".to_owned(),
            sha256: String::new(),
            page_count: 200,
        };

        let text = prompt("Find it.", &document, &part());

        assert!(text.starts_with("Find it."));
        assert!(text.contains("The document: Common Criteria, Part 3, pages 1 to 25."));
        assert!(text.ends_with("=== Page 1 ===\nCover"));
    }

    #[test]
    fn the_pages_of_the_answer_are_read_within_the_part() {
        assert_eq!(read(r#"{"first_page": 3, "last_page": 6}"#, &part()), Ok(Some((3, 6))));
        assert_eq!(read("Here: ```{\"first_page\": 2, \"last_page\": 2}```", &part()), Ok(Some((2, 2))));
        assert_eq!(read(r#"{"first_page": null, "last_page": null}"#, &part()), Ok(None));
    }

    #[test]
    fn pages_outside_the_part_or_no_pages_are_refused() {
        assert!(read(r#"{"first_page": 20, "last_page": 30}"#, &part()).is_err());
        assert!(read(r#"{"first_page": 6, "last_page": 3}"#, &part()).is_err());
        assert!(read(r#"{"first_page": "three"}"#, &part()).is_err());
        assert!(read("no JSON here", &part()).is_err());
    }
}
