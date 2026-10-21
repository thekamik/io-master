//! A quote of a document of a standard, looked for on the page it names
//! (`model/standards/quote.rs`): **nothing is taken on trust**, and a
//! passage that is not on its page stays visible as such. Used for the
//! quotes the evaluator adds and for those the AI proposes.

use crate::db::oscal::error::OscalError;
use crate::db::standards as db_standards;
use crate::db::user::database::UserDb;
use crate::model::evaluation_support::QuoteDraft;
use crate::model::standards::MAX_QUOTE;
use crate::model::standards::quote::is_on_page;

/// The quote `text` of page `page` of document `source`, with `found`
/// set by the program.
pub fn checked(db: &UserDb, source: i64, page: u32, text: &str) -> Result<QuoteDraft, OscalError> {
    let text = text.trim();

    ensure!(
        !text.is_empty() && text.chars().count() <= MAX_QUOTE,
        OscalError::Invalid(format!("a quote has 1 to {MAX_QUOTE} characters"))
    );

    let Some(page_text) = db_standards::read::page(db, source, page)? else {
        return Err(OscalError::Invalid(format!("the document of the standard has no page {page}")));
    };

    Ok(QuoteDraft { source_document_id: source, page, text: text.to_owned(), found: is_on_page(text, &page_text) })
}
