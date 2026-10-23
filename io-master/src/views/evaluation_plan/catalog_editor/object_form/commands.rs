//! The commands of the object chosen.

use serde_json::Value;
use tauri::State;

use crate::app::error::AppError;
use crate::app::state::AppState;
use crate::db::evaluation_support as db_evaluation_support;
use crate::db::oscal as db_oscal;
use crate::db::oscal::error::OscalError;
use crate::model::oscal::catalog::{Control, Group};

use super::super::oscal_object;
use super::view::ObjectView;

/// The group or control `id` of catalog `uuid`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn object_form_get(state: State<'_, AppState>, uuid: String, id: String) -> Result<ObjectView, AppError> {
    let session = state.session()?;
    let catalog = db_oscal::read::catalog(&session.db(), &uuid)?;

    let Some(location) = catalog.locate(&id)? else {
        return Err(OscalError::NoSuchObject(id).into());
    };

    ObjectView::new(&catalog, &location, &id)
}

/// Changes the own fields of group `id` of catalog `uuid` (`group`: its
/// JSON of OSCAL, the id may be new).
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn object_form_save_group(
    state: State<'_, AppState>,
    uuid: String,
    id: String,
    group: Value,
) -> Result<(), AppError> {
    let session = state.session()?;
    let group: Group = oscal_object::read(group)?;
    let db = session.db();

    let mut catalog = db_oscal::read::catalog(&db, &uuid)?;
    catalog.update_group(&id, group)?;
    db_oscal::update::catalog(&db, catalog)?;

    Ok(())
}

/// Changes the own fields of control `id` of catalog `uuid`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn object_form_save_control(
    state: State<'_, AppState>,
    uuid: String,
    id: String,
    control: Value,
) -> Result<(), AppError> {
    let session = state.session()?;
    let control: Control = oscal_object::read(control)?;
    let db = session.db();

    let mut catalog = db_oscal::read::catalog(&db, &uuid)?;
    catalog.update_control(&id, control)?;
    db_oscal::update::catalog(&db, catalog)?;

    Ok(())
}

/// Deletes the group or control `id` of catalog `uuid` with everything it
/// holds, and the quotes of everything deleted.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn object_form_delete(state: State<'_, AppState>, uuid: String, id: String) -> Result<(), AppError> {
    let session = state.session()?;
    let db = session.db();

    let mut catalog = db_oscal::read::catalog(&db, &uuid)?;
    let gone = catalog.remove(&id)?;

    db_oscal::update::catalog(&db, catalog)?;
    db_evaluation_support::delete::quotes_of(&db, &uuid, &gone)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Unit tests of `object_form/commands.rs`, through the IPC of Tauri.

    use serde_json::json;

    use crate::views::evaluation_plan::catalog_editor::test_support::{add_control, add_group, with_catalog};

    #[test]
    fn an_object_is_read_with_its_parts_and_own_fields() {
        let (window, uuid, _) = with_catalog();
        add_group(&window, &uuid, "ADV");
        add_control(&window, &uuid, "ADV", "ADV_FSP.4").unwrap();

        let group = window.call("object_form_get", json!({ "uuid": uuid, "id": "ADV" })).unwrap();
        assert_eq!(group["kind"], "group");
        // The own fields only: what the group holds is not among them.
        assert!(group["fields"].get("controls").is_none());

        let control = window.call("object_form_get", json!({ "uuid": uuid, "id": "ADV_FSP.4" })).unwrap();
        assert_eq!(control["parts"][0]["prose"], "The developer shall provide.");
        assert_eq!(control["parts"][0]["level"], 0);
    }

    #[test]
    fn a_change_keeps_what_the_object_holds_and_a_deletion_takes_it() {
        let (window, uuid, _) = with_catalog();
        add_group(&window, &uuid, "ADV");
        add_control(&window, &uuid, "ADV", "ADV_FSP.4").unwrap();

        let renamed = json!({ "id": "ADV", "title": "Development" });
        window.call("object_form_save_group", json!({ "uuid": uuid, "id": "ADV", "group": renamed })).unwrap();
        let changed = json!({ "id": "ADV_FSP.4", "title": "Complete" });
        window
            .call("object_form_save_control", json!({ "uuid": uuid, "id": "ADV_FSP.4", "control": changed }))
            .unwrap();

        let tree = window.call("catalog_tree_get", json!({ "uuid": uuid })).unwrap();
        assert_eq!(tree["rows"][0]["title"], "Development");
        assert_eq!(tree["rows"][1]["title"], "Complete");

        window.call("object_form_delete", json!({ "uuid": uuid, "id": "ADV" })).unwrap();
        assert_eq!(window.call("catalog_tree_get", json!({ "uuid": uuid })).unwrap()["rows"], json!([]));
        assert_eq!(
            window.call("object_form_delete", json!({ "uuid": uuid, "id": "ADV" })).unwrap_err()["kind"],
            "invalid"
        );
    }

    #[test]
    fn what_is_not_oscal_is_refused() {
        let (window, uuid, _) = with_catalog();
        add_group(&window, &uuid, "ADV");

        let error =
            window.call("object_form_save_group", json!({ "uuid": uuid, "id": "ADV", "group": { "title": 5 } }));

        assert_eq!(error.unwrap_err()["kind"], "invalid");
    }
}
