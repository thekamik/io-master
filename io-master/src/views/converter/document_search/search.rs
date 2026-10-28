//! The search of the text of the documents of standards: in the one form
//! of `model/standards/quote.rs`, so that spaces, words divided at the
//! end of a line and case do not matter; the snippet is the text as it is
//! on the page.

use crate::db_old::error::DbError;
use crate::db_old::standards as db_standards;
use crate::db_old::user::database::UserDb;
use crate::model_old::standards::quote::{normalize, normalize_with_places};
use crate::model_old::standards::{Hit, MAX_HITS};

/// The longest text searched for, in characters.
pub const MAX_SEARCH: usize = 1000;

/// The characters of text around a hit of a search, on each side.
const SNIPPET: usize = 80;

/// Searches the text of the documents (all, or one), in the form of
/// `model/standards/quote.rs` (spaces, divided words, case do not matter). At most
/// [`MAX_HITS`] hits, in the order of the documents and pages.
pub fn search(db: &UserDb, text: &str, document: Option<i64>) -> Result<Vec<Hit>, DbError> {
    let wanted = normalize(text);

    if wanted.is_empty() {
        return Ok(Vec::new());
    }

    let mut hits = Vec::new();

    db_standards::read::each_page(db, document, |document_id, number, page| {
        let (normal, places) = normalize_with_places(page);

        let mut from = 0;

        while let Some(found) = normal[from..].find(&wanted) {
            let at = from + found;

            hits.push(Hit {
                document_id,
                page: number,
                snippet: snippet(page, places[at], places[at + wanted.len() - 1]),
            });

            if hits.len() >= MAX_HITS {
                return false;
            }

            from = at + wanted.len();
        }

        true
    })?;

    Ok(hits)
}

/// The text of the page around a hit that runs from byte `first` to
/// the character starting at byte `last`, as it is in the page, on
/// one line.
fn snippet(page: &str, first: usize, last: usize) -> String {
    let end = last + page[last..].chars().next().map_or(0, char::len_utf8);
    let before: String = page[..first].chars().rev().take(SNIPPET).collect::<Vec<_>>().into_iter().rev().collect();
    let after: String = page[end..].chars().take(SNIPPET).collect();
    let open = if before.len() < first { "…" } else { "" };
    let close = if end + after.len() < page.len() { "…" } else { "" };
    let text = format!("{open}{before}{}{after}{close}", &page[first..end]);

    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
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
}
