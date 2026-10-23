//! The commands of a new object.

use serde_json::Value;
use tauri::State;

use crate::app::error::AppError;
use crate::app::state::AppState;
use crate::db::oscal as db_oscal;
use crate::model::oscal::catalog::{Control, Group};

use super::super::oscal_object;
use super::view::PlaceForm;

/// Adds `group` (its JSON of OSCAL) at `place` of catalog `uuid`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn new_object_add_group(
    state: State<'_, AppState>,
    uuid: String,
    place: PlaceForm,
    group: Value,
) -> Result<(), AppError> {
    let session = state.session()?;
    let place = place.into_place()?;
    let group: Group = oscal_object::read(group)?;
    let db = session.db();

    let mut catalog = db_oscal::read::catalog(&db, &uuid)?;
    catalog.add_group(&place, group)?;
    db_oscal::update::catalog(&db, catalog)?;

    Ok(())
}

/// Adds `control` (its JSON of OSCAL) at `place` of catalog `uuid`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn new_object_add_control(
    state: State<'_, AppState>,
    uuid: String,
    place: PlaceForm,
    control: Value,
) -> Result<(), AppError> {
    let session = state.session()?;
    let place = place.into_place()?;
    let control: Control = oscal_object::read(control)?;
    let db = session.db();

    let mut catalog = db_oscal::read::catalog(&db, &uuid)?;
    catalog.add_control(&place, control)?;
    db_oscal::update::catalog(&db, catalog)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Unit tests of `new_object/commands.rs`, through the IPC of Tauri.

    use serde_json::json;

    use crate::views::evaluation_plan::catalog_editor::test_support::{add_control, add_group, with_catalog};

    #[test]
    fn an_id_is_used_once_and_a_place_must_exist() {
        let (window, uuid, _) = with_catalog();
        add_group(&window, &uuid, "ADV");
        add_control(&window, &uuid, "ADV", "ADV_FSP.4").unwrap();

        assert_eq!(add_control(&window, &uuid, "ADV", "ADV_FSP.4").unwrap_err()["kind"], "invalid");
        assert_eq!(add_control(&window, &uuid, "ATE", "ATE_IND.2").unwrap_err()["kind"], "invalid");

        let place = json!({ "kind": "elsewhere" });
        let group = json!({ "id": "X", "title": "X" });
        let error = window.call("new_object_add_group", json!({ "uuid": uuid, "place": place, "group": group }));
        assert_eq!(error.unwrap_err()["kind"], "invalid");
    }

    #[test]
    fn an_enhancement_goes_into_its_control() {
        let (window, uuid, _) = with_catalog();
        add_group(&window, &uuid, "ADV");
        add_control(&window, &uuid, "ADV", "ADV_FSP.4").unwrap();

        let enhancement = json!({ "id": "ADV_FSP.4.a", "title": "More" });
        let place = json!({ "kind": "control", "id": "ADV_FSP.4" });
        window.call("new_object_add_control", json!({ "uuid": uuid, "place": place, "control": enhancement })).unwrap();

        let tree = window.call("catalog_tree_get", json!({ "uuid": uuid })).unwrap();
        assert_eq!(tree["rows"][2]["id"], "ADV_FSP.4.a");
        assert_eq!(tree["rows"][2]["level"], 2);
    }
}
