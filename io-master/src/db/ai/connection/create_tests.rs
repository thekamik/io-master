//! Unit tests of `db/ai/connection/` (attached to `create.rs`; they
//! cover the whole folder).

use super::*;

use std::fs;

use crate::ai::api::api_key::ApiKey;
use crate::ai::api::connection::{AnswerMode, Field};
use crate::db::ai::connection::{delete, update};
use crate::db::encryption::database_key::DatabaseKey;
use crate::db::user::database as user_db;
use crate::random::random::fill_random;
use crate::tpm::test_support::TempDir;

const SECRET_KEY: &str = "sk-very-secret-key-of-anna";

/// A new database of an evaluator in a temporary directory.
fn database(directory: &TempDir) -> UserDb {
    let key = DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap();

    user_db::create(directory.path(), 1, &key).unwrap()
}

/// A connection that keeps the rules.
fn named(name: &str) -> Connection {
    Connection { name: name.to_owned(), model: "llama".to_owned(), ..Connection::template() }
}

fn key() -> ApiKey {
    ApiKey::new(SECRET_KEY.to_owned()).unwrap()
}

#[test]
fn a_connection_is_saved_read_and_changed() {
    let directory = TempDir::new();
    let db = database(&directory);

    let saved = connection(&db, &named("Local")).unwrap();
    assert_eq!(saved, Saved { id: 1, key_removed: false });

    let stored = read::get(&db, 1).unwrap().unwrap();
    assert_eq!(stored.connection, named("Local"));
    assert!(!stored.has_key);

    let changed = Connection {
        answer: AnswerMode::Whole,
        text_path: "choices[0].message.content".to_owned(),
        ..named("Local, whole")
    };
    update::connection(&db, 1, &changed).unwrap();

    assert_eq!(read::get(&db, 1).unwrap().unwrap().connection, changed);
    assert_eq!(read::list(&db).unwrap().len(), 1);
    assert!(read::get(&db, 7).unwrap().is_none());
}

#[test]
fn a_connection_that_breaks_a_rule_is_not_saved() {
    let directory = TempDir::new();
    let db = database(&directory);

    let open = Connection { url: "http://192.168.1.10/v1/chat/completions".to_owned(), ..named("Lab") };

    assert!(matches!(connection(&db, &open), Err(SaveError::Invalid(error)) if error.field == Field::Url));
    assert!(read::list(&db).unwrap().is_empty());
}

#[test]
fn names_are_unique_ignoring_case() {
    let directory = TempDir::new();
    let db = database(&directory);

    connection(&db, &named("Local")).unwrap();
    let second = connection(&db, &named("Other")).unwrap().id;

    assert!(matches!(connection(&db, &named("local")), Err(SaveError::NameTaken)));
    assert!(matches!(update::connection(&db, second, &named("LOCAL")), Err(SaveError::NameTaken)));

    // Its own name is not taken.
    update::connection(&db, second, &named("Other")).unwrap();

    assert!(matches!(update::connection(&db, 99, &named("New")), Err(SaveError::NoSuchConnection)));
}

#[test]
fn the_first_connection_becomes_active_and_another_can_be_chosen() {
    let directory = TempDir::new();
    let db = database(&directory);

    assert_eq!(read::active(&db).unwrap(), None);

    let first = connection(&db, &named("A")).unwrap().id;
    let second = connection(&db, &named("B")).unwrap().id;
    assert_eq!(read::active(&db).unwrap(), Some(first));

    assert!(update::active(&db, second).unwrap());
    assert_eq!(read::active(&db).unwrap(), Some(second));
    assert!(!update::active(&db, 99).unwrap());

    // Deleting the active one leaves none active.
    assert!(delete::connection(&db, second).unwrap());
    assert_eq!(read::active(&db).unwrap(), None);
    assert!(!delete::connection(&db, second).unwrap());
}

#[test]
fn the_key_is_stored_and_removed_with_the_connection() {
    let directory = TempDir::new();
    let db = database(&directory);
    let id = connection(&db, &named("Local")).unwrap().id;

    assert!(read::api_key(&db, id).unwrap().is_none());
    assert!(update::api_key(&db, id, &key()).unwrap());
    assert_eq!(read::api_key(&db, id).unwrap().unwrap().expose(), SECRET_KEY);
    assert!(read::get(&db, id).unwrap().unwrap().has_key);
    assert!(!update::api_key(&db, 99, &key()).unwrap());

    delete::connection(&db, id).unwrap();
    assert!(read::api_key(&db, id).unwrap().is_none());
}

#[test]
fn a_changed_address_removes_the_key() {
    let directory = TempDir::new();
    let db = database(&directory);
    let id = connection(&db, &named("Local")).unwrap().id;
    update::api_key(&db, id, &key()).unwrap();

    // Another path at the same address keeps the key.
    let other_path = Connection { url: "http://127.0.0.1:11434/api/chat".to_owned(), ..named("Local") };
    assert!(!update::connection(&db, id, &other_path).unwrap().key_removed);
    assert!(read::api_key(&db, id).unwrap().is_some());

    // Another port is another address.
    let other_port = Connection { url: "http://127.0.0.1:8080/api/chat".to_owned(), ..named("Local") };
    assert!(update::connection(&db, id, &other_port).unwrap().key_removed);
    assert!(read::api_key(&db, id).unwrap().is_none());
}

#[test]
fn a_key_is_given_out_only_for_the_address_it_is_bound_to() {
    let directory = TempDir::new();
    let db = database(&directory);
    let id = connection(&db, &named("Local")).unwrap().id;
    update::api_key(&db, id, &key()).unwrap();
    assert!(read::api_key(&db, id).unwrap().is_some());

    // A save interrupted after the address changed, before the key was
    // removed: the key stays in the file, but is never given out.
    db.connection_for_tests()
        .execute("UPDATE ai_connections SET url = 'https://elsewhere.example/v1/chat' WHERE id = ?1", [id])
        .unwrap();
    assert!(read::get(&db, id).unwrap().unwrap().has_key);
    assert!(read::api_key(&db, id).unwrap().is_none());

    // A key with no recorded address is not given out either.
    db.connection_for_tests().execute("UPDATE ai_connections SET key_origin = NULL WHERE id = ?1", [id]).unwrap();
    assert!(read::api_key(&db, id).unwrap().is_none());
}

#[test]
fn a_failed_save_leaves_nothing_behind() {
    let directory = TempDir::new();
    let db = database(&directory);
    let id = connection(&db, &named("Local")).unwrap().id;
    update::api_key(&db, id, &key()).unwrap();

    // The last statement of the save fails: the database refuses to
    // store the active connection.
    db.connection_for_tests()
        .execute_batch(
            "DELETE FROM settings WHERE name = 'ai.active_connection';
             CREATE TRIGGER refuse BEFORE INSERT ON settings BEGIN SELECT RAISE(ABORT, 'refused'); END;",
        )
        .unwrap();

    let elsewhere = Connection { url: "http://127.0.0.1:8080/api/chat".to_owned(), ..named("Moved") };
    assert!(update::connection(&db, id, &elsewhere).is_err());

    // Nothing of it was stored: the old address, and its key with it.
    let stored = read::get(&db, id).unwrap().unwrap();
    assert_eq!(stored.connection, named("Local"));
    assert!(read::api_key(&db, id).unwrap().is_some());

    assert!(connection(&db, &named("Second")).is_err());
    assert_eq!(read::list(&db).unwrap().len(), 1);
}

#[test]
fn the_key_cannot_be_read_from_the_file() {
    let directory = TempDir::new();
    let db = database(&directory);
    let id = connection(&db, &named("Local")).unwrap().id;
    update::api_key(&db, id, &key()).unwrap();
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

    db.connection_for_tests().execute_batch("ALTER TABLE ai_connections DROP COLUMN message_template").unwrap();
    drop(db);

    let db = user_db::open(directory.path(), 1, &key).unwrap();
    let id = connection(&db, &named("Local")).unwrap().id;

    assert_eq!(
        read::get(&db, id).unwrap().unwrap().connection.message_template,
        crate::ai::api::connection::DEFAULT_MESSAGE_TEMPLATE
    );
}
