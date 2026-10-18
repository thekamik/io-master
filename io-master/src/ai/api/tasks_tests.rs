//! Unit tests of `tasks.rs`.

use super::*;

use crate::model::ai::Role;

use crate::business::standards::test_support::database;

#[test]
fn every_task_has_its_role() {
    assert_eq!(Task::Chat.role(), Some(Role::Consultation));
    assert_eq!(Task::Check.role(), None);

    for task in Task::ALL {
        assert_eq!(Task::from_name(task.name()), Some(task));
    }
}

#[test]
fn a_task_without_instructions_stores_none() {
    // The tasks with instructions of their own come back with the
    // catalogs of step 41; the chat and the check have none.
    let test = database();

    for task in Task::ALL {
        assert_eq!(set_instructions(&test.db, task, "x").unwrap(), None);
        assert_eq!(instructions(&test.db, task).unwrap(), None);
    }
}
