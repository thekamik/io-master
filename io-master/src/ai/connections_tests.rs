//! Unit tests of `connections.rs`.

use super::*;

use std::fs;

use crate::ai::connection::Field;
use crate::db::database_key::DatabaseKey;
use crate::db::user_db;
use crate::random::random::fill_random;
use crate::tpm::test_support::TempDir;

const SECRET_KEY: &str = "sk-very-secret-key-of-anna";

/// A new database of an evaluator in a temporary directory.
fn database(directory: &TempDir) -> UserDb {
    let key = DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap();

    user_db::create(directory.path(), 1, &key).unwrap()
}

/// A connection that keeps the rules.
fn connection(name: &str) -> Connection {
    Connection {
        name: name.to_owned(),
        model: "llama".to_owned(),
        ..Connection::template()
    }
}

fn key() -> ApiKey {
    ApiKey::new(SECRET_KEY.to_owned()).unwrap()
}

#[test]
fn a_connection_is_saved_read_and_changed() {
    let directory = TempDir::new();
    let db = database(&directory);

    let saved = save(&db, None, &connection("Local")).unwrap();
    assert_eq!(saved, Saved { id: 1, key_removed: false });

    let stored = get(&db, 1).unwrap().unwrap();
    assert_eq!(stored.connection, connection("Local"));
    assert!(!stored.has_key);

    let changed = Connection {
        answer: AnswerMode::Whole,
        text_path: "choices[0].message.content".to_owned(),
        ..connection("Local, whole")
    };
    save(&db, Some(1), &changed).unwrap();

    assert_eq!(get(&db, 1).unwrap().unwrap().connection, changed);
    assert_eq!(list(&db).unwrap().len(), 1);
    assert!(get(&db, 7).unwrap().is_none());
}

#[test]
fn a_connection_that_breaks_a_rule_is_not_saved() {
    let directory = TempDir::new();
    let db = database(&directory);

    let open = Connection {
        url: "http://192.168.1.10/v1/chat/completions".to_owned(),
        ..connection("Lab")
    };

    assert!(matches!(save(&db, None, &open), Err(SaveError::Invalid(error)) if error.field == Field::Url));
    assert!(list(&db).unwrap().is_empty());
}

#[test]
fn names_are_unique_ignoring_case() {
    let directory = TempDir::new();
    let db = database(&directory);

    save(&db, None, &connection("Local")).unwrap();
    let second = save(&db, None, &connection("Other")).unwrap().id;

    assert!(matches!(save(&db, None, &connection("local")), Err(SaveError::NameTaken)));
    assert!(matches!(save(&db, Some(second), &connection("LOCAL")), Err(SaveError::NameTaken)));

    // Its own name is not taken.
    save(&db, Some(second), &connection("Other")).unwrap();

    assert!(matches!(save(&db, Some(99), &connection("New")), Err(SaveError::NoSuchConnection)));
}

#[test]
fn the_first_connection_becomes_active_and_another_can_be_chosen() {
    let directory = TempDir::new();
    let db = database(&directory);

    assert_eq!(active(&db).unwrap(), None);

    let first = save(&db, None, &connection("A")).unwrap().id;
    let second = save(&db, None, &connection("B")).unwrap().id;
    assert_eq!(active(&db).unwrap(), Some(first));

    assert!(set_active(&db, second).unwrap());
    assert_eq!(active(&db).unwrap(), Some(second));
    assert!(!set_active(&db, 99).unwrap());

    // Deleting the active one leaves none active.
    assert!(delete(&db, second).unwrap());
    assert_eq!(active(&db).unwrap(), None);
    assert!(!delete(&db, second).unwrap());
}

#[test]
fn the_key_is_stored_and_removed_with_the_connection() {
    let directory = TempDir::new();
    let db = database(&directory);
    let id = save(&db, None, &connection("Local")).unwrap().id;

    assert!(api_key(&db, id).unwrap().is_none());
    assert!(store_api_key(&db, id, &key()).unwrap());
    assert_eq!(api_key(&db, id).unwrap().unwrap().expose(), SECRET_KEY);
    assert!(get(&db, id).unwrap().unwrap().has_key);
    assert!(!store_api_key(&db, 99, &key()).unwrap());

    delete(&db, id).unwrap();
    assert!(api_key(&db, id).unwrap().is_none());
}

#[test]
fn a_changed_address_removes_the_key() {
    let directory = TempDir::new();
    let db = database(&directory);
    let id = save(&db, None, &connection("Local")).unwrap().id;
    store_api_key(&db, id, &key()).unwrap();

    // Another path at the same address keeps the key.
    let other_path = Connection {
        url: "http://127.0.0.1:11434/api/chat".to_owned(),
        ..connection("Local")
    };
    assert!(!save(&db, Some(id), &other_path).unwrap().key_removed);
    assert!(api_key(&db, id).unwrap().is_some());

    // Another port is another address.
    let other_port = Connection {
        url: "http://127.0.0.1:8080/api/chat".to_owned(),
        ..connection("Local")
    };
    assert!(save(&db, Some(id), &other_port).unwrap().key_removed);
    assert!(api_key(&db, id).unwrap().is_none());
}

#[test]
fn the_key_cannot_be_read_from_the_file() {
    let directory = TempDir::new();
    let db = database(&directory);
    let id = save(&db, None, &connection("Local")).unwrap().id;
    store_api_key(&db, id, &key()).unwrap();
    drop(db);

    let bytes = fs::read(user_db::path(directory.path(), 1)).unwrap();
    assert!(!bytes.windows(SECRET_KEY.len()).any(|window| window == SECRET_KEY.as_bytes()));
}

#[test]
fn the_settings_of_earlier_versions_are_removed() {
    let directory = TempDir::new();
    let key = DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap();
    let db = user_db::create(directory.path(), 1, &key).unwrap();

    db.set_secret("anthropic.api_key", SECRET_KEY).unwrap();
    db.set_setting("ai.model", "claude-opus-5-5").unwrap();
    drop(db);

    let db = user_db::open(directory.path(), 1, &key).unwrap();
    assert!(db.secret("anthropic.api_key").unwrap().is_none());
    assert!(db.setting("ai.model").unwrap().is_none());
}

#[test]
fn a_table_made_before_the_message_template_gets_the_column() {
    let directory = TempDir::new();
    let key = DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap();
    let db = user_db::create(directory.path(), 1, &key).unwrap();

    db.connection()
        .execute_batch("ALTER TABLE ai_connections DROP COLUMN message_template")
        .unwrap();
    drop(db);

    let db = user_db::open(directory.path(), 1, &key).unwrap();
    let id = save(&db, None, &connection("Local")).unwrap().id;

    assert_eq!(
        get(&db, id).unwrap().unwrap().connection.message_template,
        crate::ai::connection::DEFAULT_MESSAGE_TEMPLATE
    );
}
