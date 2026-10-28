//! Changing a conversion: its stage, the pages of its table of contents,
//! its groups.

use rusqlite::named_params;

use crate::db_old::error::DbError;
use crate::db_old::user::database::UserDb;
use crate::model_old::conversion::{GroupPages, GroupState, MAX_GROUPS, Pages, Stage};

/// Sets the stage of the conversion of catalog `uuid`.
///
/// Called by: `conversion_wizard_stage` (`catalog_editor.rs`).
pub fn stage(db: &UserDb, uuid: &str, stage: Stage) -> Result<(), DbError> {
    let changed = db.connection().execute(
        "UPDATE conversions SET stage = :stage WHERE catalog_uuid = :uuid",
        named_params! { ":uuid": uuid, ":stage": stage.name() },
    )?;

    if changed == 1 { Ok(()) } else { Err(DbError::NoRow { table: "conversions", id: 0 }) }
}

/// Sets the pages of the table of contents of the conversion of `uuid`.
///
/// Called by: `conversion_wizard_contents` (`catalog_editor.rs`), and the
/// search of the AI for the table of contents.
pub fn contents(db: &UserDb, uuid: &str, pages: Pages) -> Result<(), DbError> {
    let changed = db.connection().execute(
        "UPDATE conversions SET contents_document = :document, contents_first = :first, contents_last = :last
         WHERE catalog_uuid = :uuid",
        named_params! { ":uuid": uuid, ":document": pages.document_id, ":first": pages.first, ":last": pages.last },
    )?;

    if changed == 1 { Ok(()) } else { Err(DbError::NoRow { table: "conversions", id: 0 }) }
}

/// Replaces the groups of the conversion of `uuid` by `groups`, in their
/// order, in one transaction (the groups found when the controls start).
///
/// Called by: `conversion_wizard_stage` (`catalog_editor.rs`), entering the
/// stage of the controls.
pub fn groups(db: &UserDb, uuid: &str, groups: &[GroupPages]) -> Result<(), DbError> {
    if groups.len() > MAX_GROUPS {
        return Err(DbError::Inconsistent(format!("at most {MAX_GROUPS} groups")));
    }

    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute("DELETE FROM conversion_groups WHERE catalog_uuid = :uuid", named_params! { ":uuid": uuid })?;

    for (position, group) in groups.iter().enumerate() {
        let position = i64::try_from(position).map_err(|error| DbError::Inconsistent(error.to_string()))?;
        let (document, first, last) = match group.pages {
            Some(pages) => (Some(pages.document_id), Some(pages.first), Some(pages.last)),
            None => (None, None, None),
        };

        transaction.execute(
            "INSERT INTO conversion_groups (catalog_uuid, group_id, position, document_id, first_page, last_page, state)
             VALUES (:uuid, :group, :position, :document, :first, :last, :state)",
            named_params! {
                ":uuid": uuid,
                ":group": group.group_id,
                ":position": position,
                ":document": document,
                ":first": first,
                ":last": last,
                ":state": group.state.name(),
            },
        )?;
    }

    transaction.commit()?;

    Ok(())
}

/// Sets the pages of group `group` of the conversion of `uuid`.
///
/// Called by: `conversion_wizard_group_pages` (`catalog_editor.rs`).
pub fn group_pages(db: &UserDb, uuid: &str, group: &str, pages: Pages) -> Result<(), DbError> {
    let changed = db.connection().execute(
        "UPDATE conversion_groups SET document_id = :document, first_page = :first, last_page = :last
         WHERE catalog_uuid = :uuid AND group_id = :group",
        named_params! {
            ":uuid": uuid, ":group": group, ":document": pages.document_id, ":first": pages.first, ":last": pages.last,
        },
    )?;

    if changed == 1 { Ok(()) } else { Err(DbError::NoRow { table: "conversion_groups", id: 0 }) }
}

/// Sets the state of group `group` of the conversion of `uuid`.
///
/// Called by: `conversion_wizard_group_state` (`catalog_editor.rs`).
pub fn group_state(db: &UserDb, uuid: &str, group: &str, state: GroupState) -> Result<(), DbError> {
    let changed = db.connection().execute(
        "UPDATE conversion_groups SET state = :state WHERE catalog_uuid = :uuid AND group_id = :group",
        named_params! { ":uuid": uuid, ":group": group, ":state": state.name() },
    )?;

    if changed == 1 { Ok(()) } else { Err(DbError::NoRow { table: "conversion_groups", id: 0 }) }
}

#[cfg(test)]
mod tests {
    //! Tests of changing a conversion.

    use super::*;

    use crate::db_old::conversion::{create, read};
    use crate::test_support::standards::database;

    #[test]
    fn the_stage_and_the_contents_are_kept() {
        let test = database();
        create::conversion(&test.db, "c").unwrap();

        stage(&test.db, "c", Stage::Groups).unwrap();
        contents(&test.db, "c", Pages { document_id: 3, first: 2, last: 4 }).unwrap();

        let found = read::conversion(&test.db, "c").unwrap().unwrap();
        assert_eq!(found.stage, Stage::Groups);
        assert_eq!(found.contents, Some(Pages { document_id: 3, first: 2, last: 4 }));
        assert!(stage(&test.db, "no such", Stage::Groups).is_err());
    }

    #[test]
    fn the_groups_are_replaced_in_their_order_and_changed_one_by_one() {
        let test = database();
        create::conversion(&test.db, "c").unwrap();
        let group = |id: &str, pages| GroupPages { group_id: id.to_owned(), pages, state: GroupState::Todo };

        groups(&test.db, "c", &[group("old", None)]).unwrap();
        groups(&test.db, "c", &[group("b", Some(Pages { document_id: 1, first: 5, last: 9 })), group("a", None)])
            .unwrap();
        group_pages(&test.db, "c", "a", Pages { document_id: 1, first: 10, last: 12 }).unwrap();
        group_state(&test.db, "c", "b", GroupState::Done).unwrap();

        let found = read::groups(&test.db, "c").unwrap();
        assert_eq!(found.len(), 2);
        assert_eq!((found[0].group_id.as_str(), found[0].state), ("b", GroupState::Done));
        assert_eq!(found[1].pages, Some(Pages { document_id: 1, first: 10, last: 12 }));
        assert!(group_state(&test.db, "c", "old", GroupState::Done).is_err());
    }
}
