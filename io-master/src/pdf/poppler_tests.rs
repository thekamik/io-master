//! Unit tests of `poppler.rs`, with small PDFs made by
//! `test_support/standards.rs` and the real tools of Poppler.

use super::*;

use crate::test_support::standards::pdf;

#[test]
fn the_pages_and_the_title_are_read() {
    let file = pdf(&["First page", "Second page"]);

    assert_eq!(info(&file).unwrap(), PdfInfo { pages: 2, title: Some("Test standard".to_owned()) });
}

#[test]
fn the_text_of_a_range_of_pages_is_read() {
    let texts: Vec<String> = (1..=PART + 5).map(|number| format!("Text of page {number}")).collect();
    let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
    let file = pdf(&refs);

    let first = text_of_pages(&file, 1, PART).unwrap();
    let rest = text_of_pages(&file, PART + 1, PART + 5).unwrap();

    assert_eq!(first.len(), PART as usize);
    assert_eq!(first[0], "Text of page 1");
    assert_eq!(rest, ["Text of page 21", "Text of page 22", "Text of page 23", "Text of page 24", "Text of page 25"]);
}

#[test]
fn a_range_that_is_no_range_is_refused() {
    let file = pdf(&["Only page"]);

    assert!(matches!(text_of_pages(&file, 0, 1), Err(PdfError::Unreadable(_))));
    assert!(matches!(text_of_pages(&file, 2, 1), Err(PdfError::Unreadable(_))));
}

#[test]
fn a_file_that_is_not_a_pdf_is_refused() {
    assert!(matches!(info(b"just text"), Err(PdfError::Unreadable(_))));
}
