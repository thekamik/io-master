//! Unit tests of `standards/` (attached to `create.rs`; they cover the
//! whole folder).

use super::*;

use crate::standards::delete::delete;
use crate::standards::read::{get, list, page, pdf, search};
use crate::standards::update::update;

use std::fs;

use crate::test_projects::test_support::{PAGES, database, fields, load};

#[test]
fn a_document_is_stored_with_its_pages_and_file() {
    let test = database();
    let id = load(&test.db, b"%PDF-1.7 one");

    let document = get(&test.db, id).unwrap().unwrap();
    assert_eq!(document.fields, fields());
    assert_eq!(document.page_count, 3);
    assert_eq!(document.sha256, sha256_hex(b"%PDF-1.7 one"));
    assert_eq!(document.file_name, "cc-part3.pdf");

    assert_eq!(page(&test.db, id, 2).unwrap().as_deref(), Some(PAGES[1]));
    assert_eq!(page(&test.db, id, 4).unwrap(), None);
    assert_eq!(pdf(&test.db, id).unwrap().as_deref(), Some(&b"%PDF-1.7 one"[..]));
    assert_eq!(by_sha256(&test.db, &document.sha256).unwrap().map(|found| found.id), Some(id));
    assert_eq!(list(&test.db).unwrap().len(), 1);
}

#[test]
fn the_same_file_is_loaded_once() {
    let test = database();
    let id = load(&test.db, b"%PDF same");

    let again = add(&test.db, &fields(), "copy.pdf", b"%PDF same", &["x".to_owned()]);

    assert!(matches!(again, Err(DocumentError::Duplicate(existing)) if existing == id));
}

#[test]
fn a_document_needs_a_title_and_pages() {
    let test = database();
    let untitled = DocumentFields {
        title: " ".to_owned(),
        ..fields()
    };

    assert!(matches!(add(&test.db, &untitled, "a.pdf", b"a", &["x".to_owned()]), Err(DocumentError::Invalid(_))));
    assert!(matches!(add(&test.db, &fields(), "a.pdf", b"a", &[]), Err(DocumentError::Invalid(_))));
}

#[test]
fn the_text_is_searched_across_divided_words_and_case() {
    let test = database();
    let id = load(&test.db, b"%PDF search");

    let hits = search(&test.db, "functional SPECIFICATION", None).unwrap();

    assert_eq!(hits.len(), 2);
    assert!(hits.iter().all(|hit| hit.document_id == id && hit.page == 2));
    // The snippet is the text as it is on the page, on one line.
    assert!(hits[0].snippet.contains("ADV_FSP.4 Complete functional specification"), "{}", hits[0].snippet);

    assert!(search(&test.db, "  ", None).unwrap().is_empty());
    assert!(search(&test.db, "nothing like this", Some(id)).unwrap().is_empty());
}

#[test]
fn the_fields_are_changed_and_the_document_deleted() {
    let test = database();
    let id = load(&test.db, b"%PDF change");

    let changed = DocumentFields {
        version: "CC:2022 R2".to_owned(),
        ..fields()
    };
    assert!(update(&test.db, id, &changed).unwrap());
    assert_eq!(get(&test.db, id).unwrap().unwrap().fields, changed);

    assert!(delete(&test.db, id).unwrap());
    assert!(get(&test.db, id).unwrap().is_none());
    assert_eq!(page(&test.db, id, 1).unwrap(), None);
}

#[test]
fn the_text_cannot_be_read_from_the_file() {
    let test = database();
    load(&test.db, b"%PDF secret");
    let path = crate::db::user::database::path(test.directory.path(), 1);

    let bytes = fs::read(path).unwrap();
    assert!(!bytes.windows(17).any(|window| window == b"ADV_FSP.4 Complet"));
}
