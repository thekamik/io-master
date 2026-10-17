//! Unit tests of `chapter.rs`.

use super::*;

use crate::db::user::database::UserDb;
use crate::test_projects::analysis::parts::{DEFAULT_PART, parts};
use crate::test_projects::test_support::single::{self as schemes, on, project_of_scheme};
use crate::test_projects::test_support::{PAGES, TestDb, database, load, project_with_scheme, scheme_fields};

/// A database with the small standard and a project with one empty
/// scheme on it; returns the scheme and the document.
fn ready() -> (TestDb, i64, i64) {
    let test = database();
    let document = load(&test.db, b"%PDF analysis");
    let (_, scheme) = project_with_scheme(&test.db, document);

    (test, scheme, document)
}

/// The tiles of a scheme.
fn tiles_of(test: &TestDb, scheme: i64) -> Vec<Tile> {
    let project = schemes::existing(&test.db, scheme).unwrap().project;

    schemes::project_tiles(&test.db, project).unwrap().into_iter().filter(|tile| tile.scheme == scheme).collect()
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

/// Applies `proposal` to `scheme`, on its project opened from the
/// database.
fn apply(db: &UserDb, scheme: i64, proposal: &ChapterProposal) -> Result<Applied, StoreError> {
    on(db, project_of_scheme(db, scheme), |open| super::apply(db, open, scheme, proposal))
}

#[test]
fn the_pages_are_cut_into_parts_that_fit() {
    let (test, _, document) = ready();

    let whole = parts(&test.db, &[(document, 1, 3)], 400_000).unwrap();
    assert_eq!(whole.len(), 1);
    assert_eq!((whole[0].first_page, whole[0].last_page), (1, 3));
    assert!(whole[0].text.starts_with("=== Page 1 ===\n"));
    assert!(whole[0].text.contains(PAGES[2].trim()));

    // The smallest size: every page is a part of its own here.
    let small = parts(&test.db, &[(document, 1, 3)], 1).unwrap();
    assert_eq!(small.len(), 1, "the pages are short, so even the smallest part holds them");

    // A range past the end stops at the last page.
    let past = parts(&test.db, &[(document, 3, 9)], DEFAULT_PART).unwrap();
    assert_eq!((past[0].first_page, past[0].last_page), (3, 3));
}

#[test]
fn a_proposal_is_stored_as_proposed_with_its_quotes_checked() {
    let (test, scheme, document) = ready();
    let proposal = read(ANSWER, &part(document)).unwrap();

    let applied = apply(&test.db, scheme, &proposal).unwrap();

    assert_eq!(applied.added, 5);
    // t2 → t4 (an input to an output) and the link from the unknown t9
    // are dropped.
    assert_eq!(applied.links, 2);
    assert!(applied.dropped.iter().any(|reason| reason.contains("t2 → t4")));
    assert!(applied.dropped.iter().any(|reason| reason.contains("t9")));

    let stored = tiles_of(&test, scheme);
    assert!(stored.iter().all(|tile| tile.fields.state == ProposalState::Proposed));

    let found = |title: &str| stored.iter().find(|tile| tile.fields.title == title).unwrap().fields.sources[0].found;
    assert!(found("Confirm the functional specification"));
    // Across the divided word "specifi-\ncation".
    assert!(found("Functional specification"));
    assert!(!found("Invented"));

    // Laid out in columns by kind.
    let input = stored.iter().find(|tile| tile.fields.title == "Functional specification").unwrap();
    let output = stored.iter().find(|tile| tile.fields.kind == TileKind::Output).unwrap();
    assert!(input.fields.x < output.fields.x);
}

#[test]
fn the_same_requirement_found_again_is_one_tile() {
    let (test, scheme, document) = ready();
    apply(&test.db, scheme, &read(ANSWER, &part(document)).unwrap()).unwrap();

    // A second run proposes it again, with another quote, and links a
    // new input to it by its identifier.
    let again = r#"{"tiles": [
        {"key": "x1", "kind": "requirement", "identifier": "adv_fsp.4.1e", "title": "Other words",
         "quotes": [{"page": 2, "text": "ADV_FSP.4 Complete functional specification"}]},
        {"key": "x2", "kind": "input", "title": "Security target"}],
      "links": [{"from": "x2", "to": "ADV_FSP.4.1E"}]}"#;

    let applied = apply(&test.db, scheme, &read(again, &part(document)).unwrap()).unwrap();

    assert_eq!((applied.added, applied.joined, applied.links), (1, 1, 1));

    let stored = tiles_of(&test, scheme);
    let requirement = stored.iter().find(|tile| tile.fields.identifier == "ADV_FSP.4.1E").unwrap();
    assert_eq!(requirement.fields.sources.len(), 2);
}

#[test]
fn a_link_proposed_twice_is_one_link_without_a_message() {
    let (test, scheme, document) = ready();
    let answer = r#"{"tiles": [
        {"key": "r1", "kind": "requirement", "identifier": "9.1.4", "title": "First"},
        {"key": "r2", "kind": "requirement", "identifier": "9.1.5", "title": "Second"}],
      "links": [{"from": "r1", "to": "r2"}, {"from": "r1", "to": "r2"}]}"#;

    let applied = apply(&test.db, scheme, &read(answer, &part(document)).unwrap()).unwrap();

    // One requirement precedes the other; the repeated link is not an error.
    assert_eq!(applied.links, 1);
    assert!(applied.dropped.is_empty(), "{:?}", applied.dropped);
}

#[test]
fn a_requirement_of_another_chapter_is_linked_not_copied() {
    let (test, first, document) = ready();
    let project = schemes::existing(&test.db, first).unwrap().project;
    let second = schemes::create(&test.db, project, &scheme_fields("6", "ATE")).unwrap();

    let mut earlier = TileFields::new(TileKind::Requirement, "Confirm the specification");
    earlier.identifier = "9.1.4".to_owned();
    let earlier = schemes::add_tile(&test.db, first, &earlier).unwrap();

    // The second chapter names the requirement of the first again, and
    // a new one that needs it met first; and an input with the title of
    // an input of the first chapter, which stays its own.
    let input = schemes::add_tile(&test.db, first, &TileFields::new(TileKind::Input, "Sample")).unwrap();
    let answer = r#"{"tiles": [
        {"key": "r1", "kind": "requirement", "identifier": "9.1.4", "title": "Confirm"},
        {"key": "r2", "kind": "requirement", "identifier": "9.2.1", "title": "Test"},
        {"key": "i1", "kind": "input", "title": "Sample"}],
      "links": [{"from": "r1", "to": "r2"}, {"from": "i1", "to": "r2"}]}"#;

    let applied = apply(&test.db, second, &read(answer, &part(document)).unwrap()).unwrap();

    assert_eq!((applied.added, applied.joined, applied.links), (2, 1, 2));

    let new = tiles_of(&test, second);
    assert_eq!(new.len(), 2);
    assert!(new.iter().all(|tile| tile.id != input));

    let links = schemes::links(&test.db, project).unwrap();
    assert!(links.iter().any(|link| link.from == earlier && new.iter().any(|tile| tile.id == link.to)));
}
