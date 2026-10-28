//! The text of the evidence of an input as a question shows it (step 52 of
//! docs/history.md): every page under a line "=== Page N ===", word for word,
//! cut into parts of a bounded size by whole pages (a page is never cut).
//! Shared by the chat of an input, the fragments and the assessment.

use crate::model_old::evaluation::Evidence;

/// The most characters of evidence in one question.
pub const PART_CHARS: usize = 24_000;

/// A part of a piece of evidence: its pages from `first` to `last`
/// (counted from 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Part {
    pub first: usize,
    pub last: usize,
}

/// The parts of `evidence`: whole pages, each part of at most
/// [`PART_CHARS`] characters, or one page when it is longer alone.
///
/// Called by: `views/evaluation_lab/fragments/propose.rs`, [`excerpt`].
pub fn parts(evidence: &Evidence) -> Vec<Part> {
    let mut found = Vec::new();
    let mut start = 1;
    let mut size = 0;

    for (index, page) in evidence.pages.iter().enumerate() {
        let number = index + 1;
        let length = page.chars().count();

        if number > start && size + length > PART_CHARS {
            found.push(Part { first: start, last: number - 1 });
            start = number;
            size = 0;
        }

        size += length;
    }

    if start <= evidence.pages.len() {
        found.push(Part { first: start, last: evidence.pages.len() });
    }

    found
}

/// The pages of `part` of `evidence`, each under its line.
///
/// Called by: `evidence_fragments.rs`, [`excerpt`].
pub fn pages_text(evidence: &Evidence, part: Part) -> String {
    let mut text = String::new();

    for (index, page) in evidence.pages.iter().enumerate() {
        let number = index + 1;

        if number >= part.first && number <= part.last {
            text.push_str(&format!("=== Page {number} ===\n{page}\n"));
        }
    }

    text
}

/// The start of every piece of `evidence`, as much as a question holds: the
/// first part of each, in order, until [`PART_CHARS`] characters are used;
/// what is left out is said, never cut in the middle of a page.
///
/// Called by: `input_chat.rs`.
/// Calls: [`parts`], [`pages_text`].
pub fn excerpt(evidence: &[Evidence]) -> String {
    let mut text = String::new();
    let mut used = 0;

    for piece in evidence {
        let Some(first) = parts(piece).into_iter().next() else {
            continue;
        };
        let pages = pages_text(piece, first);
        let length = pages.chars().count();

        if used > 0 && used + length > PART_CHARS {
            text.push_str(&format!("--- \"{}\": not shown here, ask about it in a new question ---\n", piece.name));
            continue;
        }

        text.push_str(&format!("--- \"{}\" ({} pages) ---\n{pages}", piece.name, piece.pages.len()));

        if first.last < piece.pages.len() {
            text.push_str(&format!("--- the pages after {} are not shown here ---\n", first.last));
        }

        used += length;
    }

    text
}

#[cfg(test)]
mod tests {
    //! Unit tests of `evidence_text.rs`.

    use super::*;

    use crate::model_old::evaluation::EvidenceKind;

    fn evidence(name: &str, pages: &[usize]) -> Evidence {
        Evidence {
            uuid: name.to_owned(),
            block: "i".to_owned(),
            kind: EvidenceKind::Pdf,
            name: name.to_owned(),
            pages: pages.iter().map(|length| "x".repeat(*length)).collect(),
            added: 0,
        }
    }

    #[test]
    fn the_parts_hold_whole_pages_of_a_bounded_text() {
        let piece = evidence("st", &[PART_CHARS - 5, 10, PART_CHARS * 2, 3]);

        assert_eq!(
            parts(&piece),
            [
                Part { first: 1, last: 1 },
                Part { first: 2, last: 2 },
                Part { first: 3, last: 3 },
                Part { first: 4, last: 4 }
            ]
        );
        assert_eq!(parts(&evidence("short", &[3, 3])), [Part { first: 1, last: 2 }]);
        assert!(pages_text(&evidence("short", &[3, 3]), Part { first: 2, last: 2 }).starts_with("=== Page 2 ===\nxxx"));
    }

    #[test]
    fn an_excerpt_says_what_it_leaves_out() {
        let text = excerpt(&[evidence("st", &[PART_CHARS - 5, 10]), evidence("notes", &[20])]);

        assert!(text.contains("\"st\" (2 pages)"));
        assert!(text.contains("the pages after 1 are not shown"));
        assert!(text.contains("\"notes\": not shown here"));
    }
}
