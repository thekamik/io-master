//! Unit tests of `quote.rs`.

use super::*;

const PAGE: &str = "5.1 Security functional requirements\n\
The TOE shall provide an inter-\n   face that protects the stored pass-\nwords.\n\
The evaluator shall examine the “security target”  —  all of it.\n";

#[test]
fn a_quote_across_a_line_break_and_a_divided_word_is_found() {
    assert!(is_on_page("The TOE shall provide an interface that protects", PAGE));
    assert!(is_on_page("protects the stored passwords.", PAGE));
}

#[test]
fn spaces_quotation_marks_dashes_and_case_do_not_matter() {
    assert!(is_on_page("examine the \"security target\" - all of it", PAGE));
    assert!(is_on_page("THE EVALUATOR SHALL", PAGE));
}

#[test]
fn another_word_or_a_missing_word_is_not_found() {
    assert!(!is_on_page("The TOE shall provide a interface", PAGE));
    assert!(!is_on_page("The TOE provide an interface", PAGE));
    assert!(!is_on_page("   ", PAGE));
}

#[test]
fn a_dash_that_divides_no_word_stays() {
    // "-" before a line that starts with a capital or a digit is a
    // real dash, not a divided word.
    assert_eq!(normalize("see 5-\n6"), "see 5- 6");
    assert_eq!(normalize("part A -\nPart B"), "part a - part b");
}

#[test]
fn every_byte_of_the_normal_form_knows_its_place() {
    let text = "Ab  C-\nde É";
    let (normal, places) = normalize_with_places(text);

    assert_eq!(normal, "ab cde é");
    assert_eq!(places.len(), normal.len());
    // "c" is the C of the text, "d" comes after the divided line.
    assert_eq!(&text[places[3]..places[3] + 1], "C");
    assert_eq!(&text[places[4]..places[4] + 1], "d");
    assert_eq!(&text[places[7]..places[7] + 2], "É");
}

#[test]
fn soft_hyphens_and_zero_width_spaces_are_removed() {
    assert_eq!(normalize("secu\u{00AD}rity\u{200B} target"), "security target");
}
