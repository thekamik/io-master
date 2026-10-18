//! Unit tests of `roles.rs`.

use super::*;

use crate::model::ai::MemoryRule;

use crate::business::standards::test_support::database;
use crate::db::ai::connection::{create, delete, update};
use crate::model::ai::Connection;

/// A connection named `name` that needs a key.
fn connection(name: &str) -> Connection {
    Connection {
        name: name.to_owned(),
        url: "http://127.0.0.1:11434/v1/chat/completions".to_owned(),
        model: name.to_lowercase(),
        headers: "authorization: Bearer {{api_key}}".to_owned(),
        ..Connection::template()
    }
}

#[test]
fn the_roles_are_those_the_user_named() {
    let titles: Vec<&str> = list().iter().map(|role| role.title()).collect();

    assert_eq!(titles, ["Analysis of a text and its fragments", "Summary", "Consultation", "Formal analysis"]);

    for role in Role::ALL {
        assert_eq!(Role::from_name(role.name()), Some(role));
    }

    // The quotes of a standard are never shortened.
    assert_eq!(Role::TextAnalysis.memory(), MemoryRule::Verbatim);
    assert_eq!(Role::FormalAnalysis.memory(), MemoryRule::Verbatim);
    assert!(Role::TextAnalysis.quotes_checked() && !Role::Consultation.quotes_checked());
}

#[test]
fn a_role_uses_its_own_connection_else_the_active_one() {
    let test = database();
    assert!(matches!(model(&test.db, Role::Consultation), Err(ModelError::NoConnection)));

    let first = create::connection(&test.db, &connection("First")).unwrap().id;
    let second = create::connection(&test.db, &connection("Second")).unwrap().id;
    let key = ApiKey::new("sk-key".to_owned()).unwrap();
    update::api_key(&test.db, first, &key).unwrap();

    // The first connection is the active one.
    assert_eq!(model(&test.db, Role::TextAnalysis).unwrap().name, "First");

    // A role of its own: needs the key of its connection.
    assert!(set_connection(&test.db, Role::FormalAnalysis, Some(second)).unwrap());
    assert_eq!(connection_of(&test, Role::FormalAnalysis), Some(second));
    assert!(matches!(model(&test.db, Role::FormalAnalysis), Err(ModelError::NoKey(name)) if name == "Second"));

    // Its connection deleted: the role uses the active one again.
    delete::connection(&test.db, second).unwrap();
    assert_eq!(connection_of(&test, Role::FormalAnalysis), None);
    assert_eq!(model(&test.db, Role::FormalAnalysis).unwrap().name, "First");

    // A connection that does not exist is refused.
    assert!(!set_connection(&test.db, Role::Summary, Some(9_999)).unwrap());
}

fn connection_of(test: &crate::business::standards::test_support::TestDb, role: Role) -> Option<i64> {
    super::connection(&test.db, role).unwrap()
}

#[test]
fn a_check_runs_without_a_key() {
    let test = database();
    let id = create::connection(&test.db, &connection("Local")).unwrap().id;

    assert!(matches!(model_of(&test.db, id, true), Err(ModelError::NoKey(_))));
    assert!(model_of(&test.db, id, false).unwrap().api_key.is_none());
}
