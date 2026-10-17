//! Unit tests of `tasks.rs`.

use super::*;

use crate::evaluation::test_support::database;

#[test]
fn every_task_has_its_role() {
    assert_eq!(Task::Chapter.role(), Some(Role::TextAnalysis));
    assert_eq!(Task::Contents.role(), Some(Role::TextAnalysis));
    assert_eq!(Task::Chat.role(), Some(Role::Consultation));
    assert_eq!(Task::Check.role(), None);

    for task in Task::ALL {
        assert_eq!(Task::from_name(task.name()), Some(task));
    }
}

#[test]
fn instructions_are_changed_and_brought_back() {
    let test = database();

    assert_eq!(instructions(&test.db, Task::Chapter).unwrap().as_deref(), Some(analysis::DEFAULT_INSTRUCTIONS));

    assert_eq!(set_instructions(&test.db, Task::Chapter, "Mine.").unwrap().as_deref(), Some("Mine."));
    assert_eq!(instructions(&test.db, Task::Chapter).unwrap().as_deref(), Some("Mine."));
    // The other task keeps its own.
    assert_eq!(instructions(&test.db, Task::Contents).unwrap().as_deref(), Some(contents::DEFAULT_INSTRUCTIONS));

    // Empty text: the default ones again.
    let brought_back = set_instructions(&test.db, Task::Chapter, "  ").unwrap();
    assert_eq!(brought_back.as_deref(), Some(analysis::DEFAULT_INSTRUCTIONS));
    assert_eq!(instructions(&test.db, Task::Chapter).unwrap().as_deref(), Some(analysis::DEFAULT_INSTRUCTIONS));

    // A task without instructions stores none.
    assert_eq!(set_instructions(&test.db, Task::Chat, "x").unwrap(), None);
    assert_eq!(instructions(&test.db, Task::Chat).unwrap(), None);
}
