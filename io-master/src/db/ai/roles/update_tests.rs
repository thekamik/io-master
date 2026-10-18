//! Unit tests of `db/ai/roles/` (attached to `update.rs`).

use super::*;

use crate::business::standards::test_support::database;
use crate::db::ai::connection::create;
use crate::db::ai::roles::read;
use crate::model::ai::Connection;

#[test]
fn a_role_gets_a_connection_and_loses_it() {
    let test = database();
    let local = Connection {
        name: "Local".to_owned(),
        url: "http://127.0.0.1:11434/v1/chat/completions".to_owned(),
        model: "gemma4".to_owned(),
        ..Connection::template()
    };
    let id = create::connection(&test.db, &local).unwrap().id;

    assert_eq!(read::connection(&test.db, Role::Summary).unwrap(), None);
    assert!(connection(&test.db, Role::Summary, Some(id)).unwrap());
    assert_eq!(read::connection(&test.db, Role::Summary).unwrap(), Some(id));
    assert!(connection(&test.db, Role::Summary, None).unwrap());
    assert_eq!(read::connection(&test.db, Role::Summary).unwrap(), None);
    assert!(!connection(&test.db, Role::Summary, Some(id + 1)).unwrap());
}
