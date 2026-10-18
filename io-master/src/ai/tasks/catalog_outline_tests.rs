//! Unit tests of `catalog_outline.rs`, on data in memory.

use super::*;

use crate::model::standards::DocumentFields;

fn document() -> Document {
    Document {
        id: 3,
        fields: DocumentFields {
            title: "Common Criteria".to_owned(),
            part: "Part 3".to_owned(),
            version: "CC:2022".to_owned(),
        },
        file_name: "cc3.pdf".to_owned(),
        sha256: String::new(),
        page_count: 40,
    }
}

fn part() -> Part {
    Part { document_id: 3, first_page: 2, last_page: 4, text: "=== Page 2 ===\nContents".to_owned() }
}

#[test]
fn the_prompt_holds_the_instructions_the_known_groups_and_the_text() {
    let text = prompt(DEFAULT_INSTRUCTIONS, &document(), &part(), &[("ADV".to_owned(), "Development".to_owned())]);

    assert!(text.starts_with("You help to represent a document of a standard in NIST OSCAL"));
    assert!(text.contains("- ADV: Development"));
    assert!(text.contains("The document: Common Criteria, Part 3 (CC:2022), pages 2 to 4."));
    assert!(text.ends_with("Contents"));
}

#[test]
fn the_groups_are_read_with_their_parents_and_quotes() {
    let answer = r#"Here: {"groups": [
        {"identifier": "ADV", "title": "Development", "parent": null, "quote": {"page": 2, "text": "Class ADV: Development"}},
        {"identifier": "5.1", "title": "No universal default passwords", "parent": "ADV", "quote": {"page": 9, "text": "x"}},
        {"identifier": "X", "title": ""}]}"#;

    let outline = read(answer, &part()).unwrap();

    assert_eq!(outline.groups.len(), 2);
    assert_eq!(outline.groups[0].group.id.as_deref(), Some("ADV"));
    assert_eq!(outline.groups[0].quote, Some((2, "Class ADV: Development".to_owned())));
    // An identifier that is not a token gets a prefix; it stays as the label.
    assert_eq!(outline.groups[1].group.id.as_deref(), Some("id_5.1"));
    assert_eq!(outline.groups[1].group.props[0].value, "5.1");
    assert_eq!(outline.groups[1].parent.as_deref(), Some("ADV"));
    // A quote outside the part is left out.
    assert_eq!(outline.groups[1].quote, None);
    assert_eq!(outline.dropped.len(), 1);

    assert!(read("no JSON", &part()).is_err());
}
