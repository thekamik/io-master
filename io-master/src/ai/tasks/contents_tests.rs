//! Unit tests of `contents.rs`: the question and the reading of the
//! answer, on data in memory (no database, no model).

use super::*;

use crate::model::standards::DocumentFields;

/// The document whose table of contents is read.
fn document() -> Document {
    Document {
        id: 3,
        fields: DocumentFields { title: "EN 18031-1".to_owned(), part: String::new(), version: "2024".to_owned() },
        file_name: "en18031.pdf".to_owned(),
        sha256: String::new(),
        page_count: 6,
    }
}

fn chapter(number: &str, title: &str, printed_page: Option<u32>) -> ProposedChapter {
    ProposedChapter { number: number.to_owned(), title: title.to_owned(), printed_page }
}

#[test]
fn the_answer_is_read_in_its_order() {
    let answer = r#"Here: {"chapters": [
        {"number": "5.1", "title": "Access control", "page": 2},
        {"number": "5.2", "title": "Secure update", "page": null},
        {"number": "5.3", "title": ""}]}"#;

    let (chapters, dropped) = read(answer).unwrap();

    assert_eq!(chapters, [chapter("5.1", "Access control", Some(2)), chapter("5.2", "Secure update", None)]);
    assert_eq!(dropped.len(), 1);
    assert!(read("no JSON here").is_err());
}

#[test]
fn the_prompt_holds_the_instructions_and_the_table() {
    let document = document();
    let part =
        Part { document_id: document.id, first_page: 1, last_page: 1, text: "=== Page 1 ===\nContents".to_owned() };

    let text = prompt(DEFAULT_INSTRUCTIONS, &document, &part);

    assert!(text.starts_with("You help to map a standard"));
    assert!(text.contains("The table of contents of EN 18031-1, pages 1 to 1."));
    assert!(text.ends_with("=== Page 1 ===\nContents"));
}
