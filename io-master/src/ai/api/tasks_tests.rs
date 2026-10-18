//! Unit tests of `tasks.rs`.

use super::*;

use crate::model::ai::Role;

use crate::business::standards::test_support::database;

#[test]
fn every_task_has_its_role() {
    assert_eq!(Task::CatalogOutline.role(), Some(Role::TextAnalysis));
    assert_eq!(Task::CatalogSection.role(), Some(Role::TextAnalysis));
    assert_eq!(Task::Chat.role(), Some(Role::Consultation));
    assert_eq!(Task::Check.role(), None);

    for task in Task::ALL {
        assert_eq!(Task::from_name(task.name()), Some(task));
    }
}

#[test]
fn instructions_are_changed_and_brought_back() {
    let test = database();
    let default = catalog_section::DEFAULT_INSTRUCTIONS;

    assert_eq!(instructions(&test.db, Task::CatalogSection).unwrap().as_deref(), Some(default));

    assert_eq!(set_instructions(&test.db, Task::CatalogSection, "Mine.").unwrap().as_deref(), Some("Mine."));
    assert_eq!(instructions(&test.db, Task::CatalogSection).unwrap().as_deref(), Some("Mine."));
    // The other task keeps its own.
    assert_eq!(
        instructions(&test.db, Task::CatalogOutline).unwrap().as_deref(),
        Some(catalog_outline::DEFAULT_INSTRUCTIONS)
    );

    // Empty text: the default ones again.
    assert_eq!(set_instructions(&test.db, Task::CatalogSection, "  ").unwrap().as_deref(), Some(default));
    assert_eq!(instructions(&test.db, Task::CatalogSection).unwrap().as_deref(), Some(default));

    // A task without instructions stores none.
    assert_eq!(set_instructions(&test.db, Task::Chat, "x").unwrap(), None);
    assert_eq!(instructions(&test.db, Task::Chat).unwrap(), None);
}
