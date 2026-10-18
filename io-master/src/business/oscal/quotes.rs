//! The passages of the documents of a standard (PDF) the objects of an
//! OSCAL document come from: evaluation support (`model/evaluation_support.rs`),
//! linked to the document by its UUID and to an object by its id.
//!
//! ```text
//! list()     GET     /oscal/{uuid}/quotes?object={id}
//! create()   POST    /oscal/{uuid}/quotes          looked for on its page
//! delete()   DELETE  /quotes/{id}
//! ```
//!
//! **Nothing is taken on trust:** the text of a quote is looked for on
//! the page it names (`business/standards/quote.rs`), and the result is
//! kept with it; a passage that is not there stays visible as such.

use crate::business::standards;
use crate::business::standards::quote::is_on_page;
use crate::db::error::DbError;
use crate::db::evaluation_support as db_evaluation_support;
use crate::db::user::database::UserDb;
use crate::model::evaluation_support::{Quote, QuoteDraft};

use super::documents::OscalError;

/// The longest quote, in characters: a passage, not a page.
pub use crate::model::standards::MAX_QUOTE;

/// The quotes of document `uuid` (all of them, or of object `object`).
pub fn list(db: &UserDb, uuid: &str, object: Option<&str>) -> Result<Vec<Quote>, DbError> {
    db_evaluation_support::read::quotes(db, uuid, object)
}

/// The quote `text` of page `page` of the document of a standard
/// `source`, looked for on its page: `found` set by the program.
pub fn checked(db: &UserDb, source: i64, page: u32, text: &str) -> Result<QuoteDraft, OscalError> {
    let text = text.trim();

    if text.is_empty() || text.chars().count() > MAX_QUOTE {
        return Err(OscalError::Invalid(format!("a quote has 1 to {MAX_QUOTE} characters")));
    }

    let page_text = standards::documents::page(db, source, page)?
        .ok_or_else(|| OscalError::Invalid(format!("the document of the standard has no page {page}")))?;

    Ok(QuoteDraft { source_document_id: source, page, text: text.to_owned(), found: is_on_page(text, &page_text) })
}

/// Adds the quote `text` of page `page` of document `source` of a
/// standard to object `object` of OSCAL document `uuid`.
pub fn create(db: &UserDb, uuid: &str, object: &str, source: i64, page: u32, text: &str) -> Result<i64, OscalError> {
    let draft = checked(db, source, page, text)?;

    Ok(db_evaluation_support::create::quote(db, uuid, object, &draft)?)
}

/// Deletes quote `id`; returns whether it existed.
pub fn delete(db: &UserDb, id: i64) -> Result<bool, DbError> {
    db_evaluation_support::delete::quote(db, id)
}
