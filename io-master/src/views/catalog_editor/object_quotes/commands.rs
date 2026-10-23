//! The commands of the quotes of an object.

use tauri::State;

use crate::app::error::UiError;
use crate::app::state::UiState;
use crate::db::evaluation_support as db_evaluation_support;
use crate::db::oscal as db_oscal;
use crate::db::oscal::error::OscalError;
use crate::db::standards as db_standards;

use super::super::checked_quote;
use super::view::ObjectQuotesView;

/// The quotes of object `object_id` of catalog `uuid`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn object_quotes_get(
    state: State<'_, UiState>,
    uuid: String,
    object_id: String,
) -> Result<ObjectQuotesView, UiError> {
    let session = state.session()?;
    let db = session.db();

    let quotes = db_evaluation_support::read::quotes(&db, &uuid, Some(&object_id))?;

    Ok(ObjectQuotesView::new(quotes, &db_standards::read::documents(&db)?))
}

/// Adds the quote `text` of page `page` of document `source` to object
/// `object_id` of catalog `uuid`, looked for on its page.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn object_quotes_add(
    state: State<'_, UiState>,
    uuid: String,
    object_id: String,
    source: i64,
    page: u32,
    text: String,
) -> Result<(), UiError> {
    let session = state.session()?;
    let db = session.db();

    let catalog = db_oscal::read::catalog(&db, &uuid)?;
    ensure!(catalog.ids()?.contains(&object_id), OscalError::NoSuchObject(object_id).into());

    let draft = checked_quote::checked(&db, source, page, &text)?;
    db_evaluation_support::create::quote(&db, &uuid, &object_id, &draft)?;

    Ok(())
}

/// Deletes quote `id`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn object_quotes_delete(state: State<'_, UiState>, id: i64) -> Result<(), UiError> {
    let session = state.session()?;

    ensure!(
        db_evaluation_support::delete::quote(&session.db(), id)?,
        UiError::Invalid("the quote does not exist".to_owned())
    );

    Ok(())
}

// Unit tests, in their own file to keep this one readable.
#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
