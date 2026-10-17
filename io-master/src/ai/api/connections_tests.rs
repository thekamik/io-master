//! Unit tests of `connections.rs`: what is checked and normalized
//! before a connection is stored. (The rules of the stored data are
//! tested in `db/ai/connection/`.)

use super::*;

use crate::db::encryption::database_key::DatabaseKey;
use crate::db::user::database as user_db;
use crate::model::ai::Field;
use crate::security::random::fill_random;
use crate::test_support::TempDir;

fn database(directory: &TempDir) -> UserDb {
    let key = DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap();

    user_db::create(directory.path(), 1, &key).unwrap()
}

fn named(name: &str) -> Connection {
    Connection { name: name.to_owned(), model: "llama".to_owned(), ..Connection::template() }
}

#[test]
fn a_connection_that_breaks_a_rule_is_not_saved() {
    let directory = TempDir::new();
    let db = database(&directory);

    let open = Connection { url: "http://192.168.1.10/v1/chat/completions".to_owned(), ..named("Lab") };

    assert!(matches!(create(&db, &open), Err(ConnectionSaveError::Invalid(error)) if error.field == Field::Url));
    assert!(list(&db).unwrap().is_empty());
}

#[test]
fn a_connection_is_stored_in_its_normal_form() {
    let directory = TempDir::new();
    let db = database(&directory);

    let spaced = Connection { name: "  Local ".to_owned(), model: " llama ".to_owned(), ..named("x") };
    let id = create(&db, &spaced).unwrap().id;

    let stored = get(&db, id).unwrap().unwrap().connection;
    assert_eq!((stored.name.as_str(), stored.model.as_str()), ("Local", "llama"));
    assert_eq!(stored.body, Connection::template().body);

    // The name is the same whatever the spaces around it.
    assert!(matches!(create(&db, &named("Local ")), Err(ConnectionSaveError::NameTaken)));
}
