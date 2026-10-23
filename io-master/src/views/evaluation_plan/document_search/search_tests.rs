//! Unit tests of `search.rs`.

use super::*;

use crate::test_support::standards::{database, load};

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
