//! Finding a quote in the text of a page of a standard.
//!
//! Every tile of a scheme keeps the passages of the standard it comes
//! from (`test_projects/rules.rs`). A passage is a quote: text copied from a page,
//! by hand or by the AI (step 20 of `docs/todo.md`). The program checks
//! that the quote really is on that page, so that a passage the AI
//! made up is never taken for a part of the standard.
//!
//! The text of a PDF (read by `pdftotext`, `pdf_text.rs`) and a quote
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

/// The longest quote that is accepted, in characters. A source is a
/// passage, not a page.
pub const MAX_QUOTE: usize = 2000;

/// The text in the one form of the top of this file.
pub fn normalize(text: &str) -> String {
    normalize_with_places(text).0
}

/// The text in the one form of the top of this file, and for every
/// byte of it the byte of `text` it comes from: so that a place found
/// in the normal form can be shown in the text as it was (the search,
/// `standards/read.rs`, `search`).
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

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "quote_tests.rs"]
mod tests;
