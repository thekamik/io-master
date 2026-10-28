//! Where every group of a catalog starts in its document (step 50.2 of
//! docs/history.md): the heading of each group whose controls are read (the
//! groups that hold no group) is looked for in the text of the pages after
//! the table of contents, in the order of the document, as the quotes are
//! checked (`model/standards/quote.rs`). The program finds the pages itself;
//! the numbers the AI or the table of contents give are never taken on
//! trust. The evaluator corrects what was not found.

use crate::app::error::AppError;
use crate::db::standards as db_standards;
use crate::db::user::database::UserDb;
use crate::model_old::conversion::{self, GroupPages, MAX_GROUPS, Pages};
use crate::model_old::oscal::catalog::Catalog;
use crate::model_old::standards::quote::is_on_page;

/// The pages of every group of `catalog` that holds no group, from the
/// pages of the document after its table of contents `contents`.
///
/// Called by: `conversion_wizard_stage` (`catalog_editor.rs`), entering the
/// stage of the controls.
/// Calls: `Catalog::leaf_groups`, `db_standards::read`, [`find`],
/// `conversion::ranges`.
pub fn locate(db: &UserDb, catalog: &Catalog, contents: Pages) -> Result<Vec<GroupPages>, AppError> {
    let Some(document) = db_standards::read::document(db, contents.document_id)? else {
        return Err(AppError::Invalid("the document of the table of contents does not exist".to_owned()));
    };

    let leaves = catalog.leaf_groups()?;

    ensure!(leaves.len() <= MAX_GROUPS, AppError::Invalid(format!("at most {MAX_GROUPS} groups")));

    // The pages after the table of contents, read once.
    let first = contents.last.saturating_add(1);
    let mut pages = Vec::new();

    for number in first..=document.page_count {
        pages.push((number, db_standards::read::page(db, document.id, number)?.unwrap_or_default()));
    }

    let mut starts = Vec::with_capacity(leaves.len());
    let mut from = 0;

    for (id, title, label) in &leaves {
        let found = find(pages.get(from..).unwrap_or_default(), title, label.as_deref());

        match found {
            Some(offset) => {
                from += offset;

                // A short closure: Option::map, which clippy asks for here.
                starts.push((id.clone(), pages.get(from).map(|(number, _)| *number)));
            }
            None => starts.push((id.clone(), None)),
        }
    }

    Ok(conversion::ranges(&starts, document.id, document.page_count))
}

/// The first of `pages` with the heading of a group: its label and its title
/// on one page, else its title alone; `None` when neither is found.
///
/// Called by: [`locate`].
fn find(pages: &[(u32, String)], title: &str, label: Option<&str>) -> Option<usize> {
    if let Some(label) = label {
        for (index, (_, text)) in pages.iter().enumerate() {
            if is_on_page(label, text) && is_on_page(title, text) {
                return Some(index);
            }
        }
    }

    for (index, (_, text)) in pages.iter().enumerate() {
        if is_on_page(title, text) {
            return Some(index);
        }
    }

    None
}

/// The groups `found` again, each keeping the pages and the state the
/// evaluator gave it in `known` (the wizard entered the stage of the
/// controls before): only a group new to the conversion takes what was found.
///
/// Called by: `conversion_wizard_stage` (`catalog_editor.rs`).
pub fn merge(found: Vec<GroupPages>, known: &[GroupPages]) -> Vec<GroupPages> {
    let mut merged = Vec::with_capacity(found.len());

    for group in found {
        let mut kept = group;

        for old in known {
            if old.group_id == kept.group_id {
                kept = old.clone();
            }
        }

        merged.push(kept);
    }

    merged
}

#[cfg(test)]
mod tests {
    //! Unit tests of `locate.rs`, on the small standard of the tests
    //! (`test_support/standards.rs`: page 2 holds ADV_FSP.4, page 3 ATE_IND.2).

    use super::*;

    use crate::db::oscal as db_oscal;
    use crate::model_old::conversion::GroupState;
    use crate::model_old::oscal::catalog::Group;
    use crate::model_old::oscal::catalog::edit::Place;
    use crate::test_support::standards::{database, load};
    use crate::views::converter::new_catalog::made;

    fn group(id: &str, title: &str) -> Group {
        serde_json::from_value(serde_json::json!({ "id": id, "title": title })).unwrap()
    }

    #[test]
    fn the_groups_start_where_their_headings_are() {
        let test = database();
        let document = load(&test.db, b"%PDF locate");
        let uuid = made::create(&test.db, "CC Part 3", "", &[document]).unwrap().uuid;
        let mut catalog = db_oscal::read::catalog(&test.db, &uuid).unwrap();
        catalog.add_group(&Place::Top, group("adv_fsp", "Complete functional specification")).unwrap();
        catalog.add_group(&Place::Top, group("ate_ind", "Independent testing")).unwrap();
        catalog.add_group(&Place::Top, group("nowhere", "A heading no page has")).unwrap();

        let contents = Pages { document_id: document, first: 1, last: 1 };
        let groups = locate(&test.db, &catalog, contents).unwrap();

        assert_eq!(groups[0].pages, Some(Pages { document_id: document, first: 2, last: 2 }));
        assert_eq!(groups[1].pages, Some(Pages { document_id: document, first: 3, last: 3 }));
        assert_eq!(groups[2].pages, None);
    }

    #[test]
    fn what_the_evaluator_gave_is_kept() {
        let found = vec![
            GroupPages { group_id: "a".to_owned(), pages: None, state: GroupState::Todo },
            GroupPages { group_id: "b".to_owned(), pages: None, state: GroupState::Todo },
        ];
        let known = [GroupPages {
            group_id: "b".to_owned(),
            pages: Some(Pages { document_id: 1, first: 4, last: 5 }),
            state: GroupState::Done,
        }];

        let merged = merge(found, &known);

        assert_eq!(merged[0].state, GroupState::Todo);
        assert_eq!(merged[1], known[0]);
    }
}
