//! Unit tests of `stored.rs`.

use super::*;

use std::fs;

use crate::test_support::standards::{PAGES, database, fields, load};

#[test]
fn a_document_is_stored_with_its_pages_and_file() {
    let test = database();
    let id = load(&test.db, b"%PDF-1.7 one");

    let document = db_standards::read::document(&test.db, id).unwrap().unwrap();
    assert_eq!(document.fields, fields());
    assert_eq!(document.page_count, 3);
    assert_eq!(document.sha256, sha256_hex(b"%PDF-1.7 one"));
    assert_eq!(db_standards::read::page(&test.db, id, 2).unwrap().as_deref(), Some(PAGES[1]));
    assert_eq!(db_standards::read::page(&test.db, id, 4).unwrap(), None);
}

#[test]
fn the_same_file_is_loaded_once() {
    let test = database();
    let id = load(&test.db, b"%PDF same");

    let again = create(&test.db, &fields(), "copy.pdf", b"%PDF same", &["x".to_owned()]);

    assert!(matches!(again, Err(DocumentError::Duplicate(existing)) if existing == id));
}

#[test]
fn a_document_needs_a_title_and_pages() {
    let test = database();
    let untitled = DocumentFields { title: " ".to_owned(), ..fields() };

    assert!(matches!(create(&test.db, &untitled, "a.pdf", b"a", &["x".to_owned()]), Err(DocumentError::Invalid(_))));
    assert!(matches!(create(&test.db, &fields(), "a.pdf", b"a", &[]), Err(DocumentError::Invalid(_))));
}

#[test]
fn the_fields_are_changed_and_the_document_deleted() {
    let test = database();
    let id = load(&test.db, b"%PDF change");

    let changed = DocumentFields { version: "CC:2022 R2".to_owned(), ..fields() };
    update(&test.db, id, &changed).unwrap();
    assert_eq!(db_standards::read::document(&test.db, id).unwrap().unwrap().fields, changed);

    delete(&test.db, id).unwrap();
    assert!(db_standards::read::document(&test.db, id).unwrap().is_none());
    assert!(matches!(delete(&test.db, id), Err(DocumentError::NotFound)));
    assert!(matches!(update(&test.db, id, &changed), Err(DocumentError::NotFound)));
}

#[test]
fn the_text_cannot_be_read_from_the_file() {
    let test = database();
    load(&test.db, b"%PDF secret");
    let path = crate::db::user::database::path(test.directory.path(), 1);

    let bytes = fs::read(path).unwrap();
    assert!(!bytes.windows(17).any(|window| window == b"ADV_FSP.4 Complet"));
}
