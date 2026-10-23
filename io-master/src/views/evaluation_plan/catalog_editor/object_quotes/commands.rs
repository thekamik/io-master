//! The commands of the quotes of an object.

use tauri::State;

use crate::app::error::AppError;
use crate::app::state::AppState;
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
    state: State<'_, AppState>,
    uuid: String,
    object_id: String,
) -> Result<ObjectQuotesView, AppError> {
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
    state: State<'_, AppState>,
    uuid: String,
    object_id: String,
    source: i64,
    page: u32,
    text: String,
) -> Result<(), AppError> {
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
pub fn object_quotes_delete(state: State<'_, AppState>, id: i64) -> Result<(), AppError> {
    let session = state.session()?;

    ensure!(
        db_evaluation_support::delete::quote(&session.db(), id)?,
        AppError::Invalid("the quote does not exist".to_owned())
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Unit tests of `object_quotes/commands.rs`, through the IPC of Tauri.

    use serde_json::json;

    use crate::views::evaluation_plan::catalog_editor::test_support::{add_control, add_group, with_catalog};

    #[test]
    fn quotes_are_looked_for_on_their_pages_and_go_with_their_object() {
        let (window, uuid, source) = with_catalog();
        add_group(&window, &uuid, "ADV");
        add_control(&window, &uuid, "ADV", "ADV_FSP.4").unwrap();
        let add = |text: &str| {
            window.call(
                "object_quotes_add",
                json!({ "uuid": uuid, "objectId": "ADV_FSP.4", "source": source, "page": 2, "text": text }),
            )
        };

        add("The developer shall provide a functional specification.").unwrap();
        add("The evaluator shall dance.").unwrap();
        assert_eq!(add("").unwrap_err()["kind"], "invalid");

        let quotes = window.call("object_quotes_get", json!({ "uuid": uuid, "objectId": "ADV_FSP.4" })).unwrap();
        assert_eq!(quotes["quotes"][0]["found"], true, "across the divided word");
        assert_eq!(quotes["quotes"][1]["found"], false, "made up");
        assert_eq!(quotes["quotes"][0]["document"], "Common Criteria — Part 3");
        assert_eq!(quotes["first_source"], source);

        // A document of a standard that a quote names stays.
        assert_eq!(window.call("standard_documents_delete", json!({ "id": source })).unwrap_err()["kind"], "invalid");

        let id = quotes["quotes"][1]["id"].clone();
        window.call("object_quotes_delete", json!({ "id": id })).unwrap();

        window.call("object_form_delete", json!({ "uuid": uuid, "id": "ADV" })).unwrap();
        let after = window.call("object_quotes_get", json!({ "uuid": uuid, "objectId": "ADV_FSP.4" })).unwrap();
        assert_eq!(after["quotes"], json!([]));
    }

    #[test]
    fn a_quote_of_no_object_is_refused() {
        let (window, uuid, source) = with_catalog();

        let error = window
            .call(
                "object_quotes_add",
                json!({ "uuid": uuid, "objectId": "NONE", "source": source, "page": 1, "text": "Part 3" }),
            )
            .unwrap_err();

        assert_eq!(error["kind"], "invalid");
    }
}
