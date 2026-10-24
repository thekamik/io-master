//! Finding a quote in the text of a page of a standard.
//!
//! Every requirement keeps the passages of the standard it comes from
//! (the evaluation support of step 41). A passage is a quote: text copied from a page,
//! by hand or by the AI (step 20 of `docs/todo.md`). The program checks
//! that the quote really is on that page, so that a passage the AI
//! made up is never taken for a part of the standard.
//!
//! The text of a PDF (read by `pdftotext`, `pdf/poppler.rs`) and a quote
//! rarely match character for character: lines are broken where the
//! page ends them, words are divided at the end of a line, quotation
//! marks and dashes come in several forms. Both texts are therefore
//! brought to one form before they are compared:
//!
//! ```text
//! "inter-\nface"      → "interface"     a word divided at a line end
//! "a\n  b\tc"         → "a b c"         every run of spaces is one space
//! “ ” ‘ ’ „           → " '             one form of quotation mark
//! – — ‐ ‑             → -               one form of dash
//! U+00AD, U+200B      → (removed)       soft hyphen, zero-width space
//! upper case          → lower case
//! ```
//!
//! Nothing else is changed: a quote with another word, or a word
//! left out, is not found.

/// The text in the one form of the top of this file.
pub fn normalize(text: &str) -> String {
    normalize_with_places(text).0
}

/// The text in the one form of the top of this file, and for every
/// byte of it the byte of `text` it comes from: so that a place found
/// in the normal form can be shown in the text as it was (the search,
/// `views/converter/document_search/search.rs`).
pub fn normalize_with_places(text: &str) -> (String, Vec<usize>) {
    let mut out = String::with_capacity(text.len());
    let mut places = Vec::with_capacity(text.len());
    let mut chars = text.char_indices().peekable();
    // Whether a space is due before the next visible character.
    let mut space = false;

    while let Some((at, character)) = chars.next() {
        let character = match character {
            '\u{00AD}' | '\u{200B}' | '\u{FEFF}' => continue,
            '“' | '”' | '„' | '«' | '»' => '"',
            '‘' | '’' | '‚' => '\'',
            '–' | '—' | '‐' | '‑' | '−' => '-',
            other => other,
        };

        // A word divided at the end of a line: "inter-\n  face".
        if character == '-' && chars.peek().map(|(_, next)| *next) == Some('\n') {
            let mut ahead = chars.clone();
            ahead.next();

            while ahead.peek().is_some_and(|(_, next)| *next == ' ' || *next == '\t') {
                ahead.next();
            }

            if ahead.peek().is_some_and(|(_, next)| next.is_alphabetic() && next.is_lowercase()) {
                chars = ahead;
                continue;
            }
        }

        if character.is_whitespace() {
            space = !out.is_empty();
            continue;
        }

        if space {
            out.push(' ');
            places.push(at);
            space = false;
        }

        for lower in character.to_lowercase() {
            out.push(lower);
            places.extend(std::iter::repeat_n(at, lower.len_utf8()));
        }
    }

    (out, places)
}

/// Whether `quote` is in `page`, in the form of [`normalize`]. An
/// empty quote is never found.
pub fn is_on_page(quote: &str, page: &str) -> bool {
    let quote = normalize(quote);

    !quote.is_empty() && normalize(page).contains(&quote)
}

#[cfg(test)]
mod tests {
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
}
