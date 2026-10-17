//! Unit tests of `chapter.rs`: the question and the reading of the
//! answer, on data in memory (no database, no model).

use super::*;

use crate::model::standards::DocumentFields;
use crate::model::test_projects::SchemeFields;

/// A document of a standard, as it is loaded.
fn document() -> Document {
    Document {
        id: 7,
        fields: DocumentFields {
            title: "Common Criteria".to_owned(),
            part: "Part 3".to_owned(),
            version: "CC:2022 R1".to_owned(),
        },
        file_name: "cc3.pdf".to_owned(),
        sha256: String::new(),
        page_count: 3,
    }
}

/// The scheme of chapter 5 ADV.
fn scheme() -> Scheme {
    Scheme {
        id: 1,
        project: 1,
        fields: SchemeFields {
            number: "5".to_owned(),
            name: "ADV".to_owned(),
            pages: None,
            state: ProposalState::Accepted,
        },
    }
}

/// The part of pages 2 to 3.
fn part(document: i64) -> Part {
    Part { document_id: document, first_page: 2, last_page: 3, text: String::new() }
}

const ANSWER: &str = r#"Here is the scheme:
```json
{"tiles": [
  {"key": "t1", "kind": "requirement", "identifier": "ADV_FSP.4.1E", "title": "Confirm the functional specification",
   "place": "ADV_FSP.4", "quotes": [{"page": 2, "text": "The evaluator shall confirm that the information provided meets all requirements."}]},
  {"key": "t2", "kind": "input", "title": "Functional specification",
   "quotes": [{"page": 2, "text": "The developer shall provide a functional specification."}]},
  {"key": "t3", "kind": "input", "title": "Flaw remediation procedures",
   "external": {"document": "CC Part 3", "place": "ALC_FLR.2"}},
  {"key": "t4", "kind": "output", "title": "Report of ADV_FSP"},
  {"key": "t5", "kind": "requirement", "title": "Invented", "quotes": [{"page": 3, "text": "The evaluator shall dance."}]},
  {"key": "t6", "kind": "nonsense", "title": "x"}
 ],
 "links": [{"from": "t2", "to": "t1"}, {"from": "t1", "to": "t4"}, {"from": "t2", "to": "t4"}, {"from": "t9", "to": "t1"}]}
```"#;

#[test]
fn the_prompt_holds_the_instructions_the_chapter_the_known_tiles_and_the_text() {
    let document = document();
    let scheme = scheme();

    let mut known = TileFields::new(TileKind::Requirement, "Confirm");
    known.identifier = "ADV_FSP.4.1E".to_owned();
    let mut elsewhere = TileFields::new(TileKind::Requirement, "Test a subset");
    elsewhere.identifier = "ATE_IND.2.1E".to_owned();
    let other_input = TileFields::new(TileKind::Input, "Not named");

    let text = prompt(
        DEFAULT_INSTRUCTIONS,
        &document,
        &Part { text: "=== Page 2 ===\nTEXT".to_owned(), ..part(document.id) },
        &scheme,
        &[Tile { id: 1, scheme: scheme.id, fields: known }],
        &[Tile { id: 2, scheme: 99, fields: elsewhere }, Tile { id: 3, scheme: 99, fields: other_input }],
    );

    assert!(text.starts_with("You help to build an evaluation scheme"));
    assert!(text.contains("The chapter analysed: 5 ADV."));
    assert!(text.contains("- requirement ADV_FSP.4.1E: Confirm"));
    // Of the other chapters only the requirements are named.
    assert!(text.contains("- requirement ATE_IND.2.1E: Test a subset"));
    assert!(!text.contains("Not named"));
    assert!(text.contains("Common Criteria, Part 3 (CC:2022 R1), pages 2 to 3."));
    assert!(text.ends_with("=== Page 2 ===\nTEXT"));
}

#[test]
fn an_answer_is_read_and_its_bad_parts_dropped() {
    let proposal = read(ANSWER, &part(7)).unwrap();

    assert_eq!(proposal.tiles.len(), 5);
    assert_eq!(proposal.tiles[0].fields.state, ProposalState::Proposed);
    assert_eq!(proposal.tiles[0].fields.sources[0].document_id, 7);
    assert!(proposal.tiles[2].fields.external.is_some());
    assert_eq!(proposal.tiles[3].fields.sections, OutputSection::ALL);
    assert_eq!(proposal.links.len(), 4);
    assert!(proposal.dropped.iter().any(|reason| reason.starts_with("t6")));

    assert!(read("no json at all", &part(7)).is_err());
}

#[test]
fn a_quote_outside_the_part_is_dropped() {
    let answer =
        r#"{"tiles": [{"key": "a", "kind": "requirement", "title": "R", "quotes": [{"page": 1, "text": "Scope"}]}]}"#;
    let proposal = read(answer, &part(7)).unwrap();

    assert!(proposal.tiles[0].fields.sources.is_empty());
    assert!(proposal.dropped[0].contains("outside the part"));
}
