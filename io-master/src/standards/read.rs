//! Reading the documents of standards: the list, one document, a page,
//! the PDF, and the search in their text.

use crate::db::error::DbError;
use crate::db::standards::read;
use crate::db::user::database::UserDb;

use super::model::{Document, Hit, MAX_HITS};
use super::quote::{normalize, normalize_with_places};

/// The characters of text around a hit of a search, on each side.
const SNIPPET: usize = 80;

/// Every document, by title and part.
pub fn list(db: &UserDb) -> Result<Vec<Document>, DbError> {
    read::documents(db)
}

/// One document, or `None`.
pub fn get(db: &UserDb, id: i64) -> Result<Option<Document>, DbError> {
    read::document(db, id)
}

/// The document of a file, by its SHA-256 (lowercase hex).
pub fn by_sha256(db: &UserDb, sha256: &str) -> Result<Option<Document>, DbError> {
    read::by_sha256(db, sha256)
}

/// The text of a page (counted from 1), or `None` when there is no
/// such page.
pub fn page(db: &UserDb, id: i64, number: u32) -> Result<Option<String>, DbError> {
    read::page(db, id, number)
}

/// The PDF of a document, or `None`.
pub fn pdf(db: &UserDb, id: i64) -> Result<Option<Vec<u8>>, DbError> {
    read::pdf(db, id)
}

/// Searches the text of the documents (all, or one), in the form of
/// `quote.rs` (spaces, divided words, case do not matter). At most
/// [`MAX_HITS`] hits, in the order of the documents and pages.
pub fn search(db: &UserDb, text: &str, document: Option<i64>) -> Result<Vec<Hit>, DbError> {
    let wanted = normalize(text);

    if wanted.is_empty() {
        return Ok(Vec::new());
    }

    let mut hits = Vec::new();

    read::each_page(db, document, |document_id, number, page| {
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
