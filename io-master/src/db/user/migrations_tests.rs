//! Unit tests of `migrations.rs`.

use super::*;

use crate::db::encryption::database_key::DatabaseKey;
use crate::db::user::database::{self as user_db, UserDb};
use crate::security::random::fill_random;
use crate::test_support::TempDir;

/// A new database, and the key that opens it again.
fn database(directory: &TempDir) -> (UserDb, DatabaseKey) {
    let key = DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap();
    let db = user_db::create(directory.path(), 1, &key).unwrap();
    let again = DatabaseKey::fill_with(|bytes| {
        bytes.copy_from_slice(key.expose());
        Ok::<(), ()>(())
    })
    .unwrap();

    (db, again)
}

fn version(db: &UserDb) -> i64 {
    db.connection_for_tests().query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap()
}

#[test]
fn a_new_database_is_at_the_latest_version() {
    let directory = TempDir::new();
    let (db, _) = database(&directory);

    assert_eq!(version(&db), VERSION);
}

#[test]
fn version_2_removes_what_earlier_versions_kept() {
    let directory = TempDir::new();
    let (db, key) = database(&directory);

    crate::db::user::update::set_secret(&db, "anthropic.api_key", "sk-old").unwrap();
    crate::db::user::update::set_setting(&db, "ai.model", "claude-opus-5-5").unwrap();
    db.connection_for_tests().execute_batch("PRAGMA user_version = 1").unwrap();
    drop(db);

    let db = user_db::open(directory.path(), 1, &key).unwrap();
    assert!(crate::db::user::read::secret(&db, "anthropic.api_key").unwrap().is_none());
    assert!(crate::db::user::read::setting(&db, "ai.model").unwrap().is_none());
    assert_eq!(version(&db), VERSION);
}

#[test]
fn version_2_adds_the_message_template() {
    let directory = TempDir::new();
    let (db, key) = database(&directory);

    db.connection_for_tests()
        .execute_batch("ALTER TABLE ai_connections DROP COLUMN message_template; PRAGMA user_version = 1;")
        .unwrap();
    drop(db);

    let db = user_db::open(directory.path(), 1, &key).unwrap();
    let has_template: bool = db
        .connection_for_tests()
        .query_row(
            "SELECT COUNT(*) > 0 FROM pragma_table_info('ai_connections') WHERE name = 'message_template'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(has_template);
}

#[test]
fn names_are_unique_whatever_their_letter_case() {
    let directory = TempDir::new();
    let (db, _) = database(&directory);
    let insert = |name: &str| {
        db.connection_for_tests().execute(
            "INSERT INTO projects (name, version, description, author, ready) VALUES (?1, '', '', '', 0)",
            [name],
        )
    };

    insert("ADV and ATE").unwrap();
    assert!(insert("adv AND ate").is_err());
}

#[test]
fn a_database_at_the_latest_version_runs_nothing() {
    let directory = TempDir::new();
    let (db, key) = database(&directory);

    // Something that version 2 would remove: kept, since it does not run.
    crate::db::user::update::set_setting(&db, "ai.model", "kept").unwrap();
    drop(db);

    let db = user_db::open(directory.path(), 1, &key).unwrap();
    assert_eq!(crate::db::user::read::setting(&db, "ai.model").unwrap().as_deref(), Some("kept"));
}
