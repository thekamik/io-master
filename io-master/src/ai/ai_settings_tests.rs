//! Unit tests of `ai_settings.rs`.

use super::*;

use std::fs;

use crate::db::database_key::DatabaseKey;
use crate::db::user_db;
use crate::random::random::fill_random;
use crate::tpm::test_support::TempDir;

const SECRET_KEY: &str = "sk-ant-api03-very-secret-key";

/// A new database of an evaluator in a temporary directory.
fn database(directory: &TempDir) -> (UserDb, DatabaseKey) {
    let key = DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap();
    let db = user_db::create(directory.path(), 1, &key).unwrap();

    (db, key)
}

#[test]
fn the_key_is_stored_replaced_and_removed() {
    let directory = TempDir::new();
    let (db, _) = database(&directory);

    assert!(api_key(&db).unwrap().is_none());
    assert!(!has_api_key(&db).unwrap());

    store_api_key(&db, &ApiKey::new("sk-ant-first".to_owned()).unwrap()).unwrap();
    store_api_key(&db, &ApiKey::new(SECRET_KEY.to_owned()).unwrap()).unwrap();

    assert_eq!(api_key(&db).unwrap().unwrap().expose(), SECRET_KEY);
    assert!(has_api_key(&db).unwrap());

    remove_api_key(&db).unwrap();
    assert!(api_key(&db).unwrap().is_none());
}

#[test]
fn the_key_cannot_be_read_from_the_file_or_with_another_key() {
    let directory = TempDir::new();
    let (db, _) = database(&directory);

    store_api_key(&db, &ApiKey::new(SECRET_KEY.to_owned()).unwrap()).unwrap();
    drop(db);

    // The file holds no readable trace of the key.
    let bytes = fs::read(user_db::path(directory.path(), 1)).unwrap();
    assert!(!bytes.windows(SECRET_KEY.len()).any(|window| window == SECRET_KEY.as_bytes()));
    assert!(!bytes.windows(10).any(|window| window == b"anthropic."));

    // And another database key does not open the database.
    let other = DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap();
    assert!(user_db::open(directory.path(), 1, &other).is_err());
}

#[test]
fn the_model_is_stored_and_defaults_to_opus() {
    let directory = TempDir::new();
    let (db, _) = database(&directory);

    assert_eq!(model(&db).unwrap(), Model::Opus);

    store_model(&db, Model::Haiku).unwrap();
    assert_eq!(model(&db).unwrap(), Model::Haiku);

    // A model this version no longer offers.
    db.set_setting(MODEL, "claude-old-1").unwrap();
    assert_eq!(model(&db).unwrap(), Model::DEFAULT);
}
