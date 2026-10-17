//! Unit tests of `pdf_text.rs`, with small PDFs made by
//! `test_support.rs` and the real tools of Poppler.

use super::*;

use crate::test_projects::test_support::pdf;

#[test]
fn the_pages_and_the_title_are_read() {
    let file = pdf(&["First page", "Second page"]);

    assert_eq!(
        info(&file).unwrap(),
        PdfInfo {
            pages: 2,
            title: Some("Test standard".to_owned())
        }
    );
}

#[test]
fn the_text_is_read_page_by_page_in_parts() {
    let texts: Vec<String> = (1..=PART + 5).map(|number| format!("Text of page {number}")).collect();
    let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
    let file = pdf(&refs);

    let mut reports = Vec::new();
    let pages = pages(&file, PART + 5, |done, all| {
        reports.push((done, all));
        true
    })
    .unwrap();

    assert_eq!(pages.len(), (PART + 5) as usize);
    assert_eq!(pages[0], "Text of page 1");
    assert_eq!(pages[PART as usize], format!("Text of page {}", PART + 1));
    assert_eq!(reports, [(PART, PART + 5), (PART + 5, PART + 5)]);
}

#[test]
fn the_work_can_be_stopped() {
    let texts: Vec<String> = (1..=PART + 1).map(|number| format!("Page {number}")).collect();
    let refs: Vec<&str> = texts.iter().map(String::as_str).collect();

    assert_eq!(pages(&pdf(&refs), PART + 1, |_, _| false), Err(PdfError::Stopped));
}

#[test]
fn a_file_that_is_not_a_pdf_is_refused() {
    assert!(matches!(info(b"just text"), Err(PdfError::Unreadable(_))));
}
