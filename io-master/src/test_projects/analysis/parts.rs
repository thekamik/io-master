//! The pages of a standard cut into parts that fit the context of a
//! model (local models have small ones): the text a task of the AI
//! sends, part by part (`chapter.rs`, `contents.rs`).

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::standards;

/// The size of a part when the user gives none, in characters: about
/// 3000 tokens, which leaves room for the answer even in the context
/// of a small local model.
pub const DEFAULT_PART: usize = 12_000;

/// The smallest and largest size of a part the user may choose.
pub const PART_RANGE: (usize, usize) = (2_000, 400_000);

/// A part of the text of one document, for one request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    pub document_id: i64,
    pub first_page: u32,
    pub last_page: u32,
    /// The pages, each after a line "=== Page N ===".
    pub text: String,
}

/// The pages `first` to `last` of the documents, cut into parts of at
/// most `size` characters (0: [`DEFAULT_PART`]; a page longer than a
/// part is a part of its own). `ranges` gives (document, first page,
/// last page) in order.
pub fn parts(db: &UserDb, ranges: &[(i64, u32, u32)], size: usize) -> Result<Vec<Part>, DbError> {
    let size = if size == 0 { DEFAULT_PART } else { size }.clamp(PART_RANGE.0, PART_RANGE.1);
    let mut parts = Vec::new();

    for &(document, first, last) in ranges {
        let mut current: Option<Part> = None;

        for number in first..=last {
            let Some(page) = standards::read::page(db, document, number)? else {
                break;
            };

            let block = format!("=== Page {number} ===\n{}\n", page.trim());

            match &mut current {
                Some(part) if part.text.chars().count() + block.chars().count() <= size => {
                    part.text.push_str(&block);
                    part.last_page = number;
                }
                _ => {
                    if let Some(done) = current.take() {
                        parts.push(done);
                    }

                    current = Some(Part {
                        document_id: document,
                        first_page: number,
                        last_page: number,
                        text: block,
                    });
                }
            }
        }

        parts.extend(current);
    }

    Ok(parts)
}
