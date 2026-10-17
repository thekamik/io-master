//! Unit tests of `contents.rs`.

use super::*;

use crate::db::user::database::UserDb;
use crate::model::standards::DocumentFields;
use crate::test_projects::test_support::single::{self as schemes, create_project, on};
use crate::test_projects::test_support::{database, project_fields, scheme_fields};

/// A standard of six pages: the table of contents on page 1, a
/// foreword on page 2, so that the printed page numbers are the real
/// ones minus 2.
const PAGES: [&str; 6] = [
    "Contents\n1 Scope .... 1\n5 Requirements .... 2\n5.1 Access control .... 2\n5.2 Secure update .... 3\n\
     5.3 Logging .... 4",
    "Foreword\nThis standard was prepared by a committee.",
    "1 Scope\nThis standard applies to radio equipment.",
    "5 Requirements\n5.1 Access control\nThe equipment shall control access.",
    "5.2 Secure\nupdate\nThe equipment shall verify updates.",
    "Annex A\nThe equipment shall log events (see 5.1 Access control).",
];

fn document(db: &UserDb) -> Document {
    let pages: Vec<String> = PAGES.iter().map(|page| (*page).to_owned()).collect();
    let fields = DocumentFields { title: "EN 18031-1".to_owned(), part: String::new(), version: "2024".to_owned() };
    let id = standards::create::add(db, &fields, "en18031.pdf", b"%PDF contents", &pages).unwrap();

    standards::read::get(db, id).unwrap().unwrap()
}

fn chapter(number: &str, title: &str, printed_page: Option<u32>) -> ProposedChapter {
    ProposedChapter { number: number.to_owned(), title: title.to_owned(), printed_page }
}

/// Applies the chapters to `project`, opened from the database.
fn apply(
    db: &UserDb,
    project: i64,
    chapters: &[ProposedChapter],
    spans: &[Option<PageSpan>],
) -> Result<AppliedChapters, StoreError> {
    on(db, project, |open| super::apply(db, open, chapters, spans))
}

#[test]
fn chapters_are_found_by_their_headings_and_the_rest_by_the_offset() {
    let test = database();
    let document = document(&test.db);

    let chapters = [
        chapter("5.1", "Access control", Some(2)),
        // Its heading is broken over two lines: still found.
        chapter("5.2", "Secure update", Some(3)),
        // Not in the text as a heading: the printed page 4, moved by
        // the offset of the others (+2), is page 6.
        chapter("5.3", "Logging", Some(4)),
        // Neither found nor printed.
        chapter("5.4", "Privacy", None),
    ];

    let starts = locate(&test.db, &document, 1, &chapters).unwrap();
    assert_eq!(starts, [Some(4), Some(5), Some(6), None]);

    let spans = spans(&document, &starts);
    let range = |index: usize| spans[index].map(|span| (span.first_page, span.last_page));

    assert_eq!(range(0), Some((4, 5)));
    assert_eq!(range(1), Some((5, 6)));
    // The last one found runs to the end of the document.
    assert_eq!(range(2), Some((6, 6)));
    assert_eq!(range(3), None);
}

#[test]
fn the_table_of_contents_itself_is_not_taken_for_a_heading() {
    let test = database();
    let document = document(&test.db);

    // Read from page 0 the table of contents would match; after it, the
    // real heading is found.
    let starts = locate(&test.db, &document, 1, &[chapter("1", "Scope", Some(1))]).unwrap();

    assert_eq!(starts, [Some(3)]);
}

#[test]
fn the_chapters_become_proposed_schemes_and_known_numbers_are_left_out() {
    let test = database();
    let document = document(&test.db);
    let project = create_project(&test.db, &project_fields("EN 18031-1", vec![document.id])).unwrap();
    schemes::create(&test.db, project, &scheme_fields("5.1", "Access control")).unwrap();

    let chapters = [chapter("5.1", "Access control", Some(2)), chapter("5.2", "Secure update", Some(3))];
    let spans = [Some(PageSpan { document_id: document.id, first_page: 4, last_page: 5 }), None];

    let applied = apply(&test.db, project, &chapters, &spans).unwrap();

    assert_eq!((applied.added, applied.known, applied.without_pages), (1, 1, 1));

    let stored = schemes::of_project(&test.db, project).unwrap();
    assert_eq!(stored.len(), 2);
    assert_eq!(stored[1].fields.number, "5.2");
    assert_eq!(stored[1].fields.state, ProposalState::Proposed);
    assert_eq!(stored[1].fields.pages, None);
}
