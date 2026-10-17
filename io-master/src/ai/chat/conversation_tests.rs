//! Unit tests of `conversation.rs`.

use super::*;

use crate::ai::api::api_key::ApiKey;
use crate::ai::api::connection::Connection;
use crate::db::ai::connection::{create as connections_create, update as connections_update};
use crate::ai::api::completions::Speaker;
use crate::db::encryption::database_key::DatabaseKey;
use crate::db::user::database as user_db;
use crate::random::random::fill_random;
use crate::tpm::test_support::TempDir;

/// A connection whose header uses the API key.
fn with_key_header() -> Connection {
    Connection {
        name: "Local".to_owned(),
        model: "llama".to_owned(),
        headers: "authorization: Bearer {{api_key}}".to_owned(),
        ..Connection::template()
    }
}

/// A database of an evaluator with an active connection that uses an
/// API key, and the key when `with_key`.
fn database(directory: &TempDir, with_key: bool) -> UserDb {
    let key = DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap();
    let db = user_db::create(directory.path(), 1, &key).unwrap();
    let id = connections_create::connection(&db, &with_key_header()).unwrap().id;

    if with_key {
        connections_update::api_key(&db, id, &ApiKey::new("sk-test".to_owned()).unwrap()).unwrap();
    }

    db
}

#[test]
fn a_first_message_starts_a_chat() {
    let directory = TempDir::new();
    let db = database(&directory, true);

    let prepared = prepare(&db, None, "  What is ADV_FSP?  ").unwrap();

    assert_eq!(prepared.title, "What is ADV_FSP?");
    assert_eq!(prepared.model.connection.model, "llama");
    assert_eq!(prepared.model.api_key.unwrap().expose(), "sk-test");
    assert_eq!(
        prepared.turns,
        [Turn {
            speaker: Speaker::Evaluator,
            text: "What is ADV_FSP?".to_owned()
        }]
    );
    assert_eq!(chats::list(&db).unwrap().len(), 1);
}

#[test]
fn a_further_message_continues_the_chat() {
    let directory = TempDir::new();
    let db = database(&directory, true);

    let first = prepare(&db, None, "first").unwrap();
    let second = prepare(&db, Some(first.chat_id), "second").unwrap();

    assert_eq!(second.chat_id, first.chat_id);
    assert_eq!(second.turns.len(), 2);
    assert_eq!(chats::list(&db).unwrap().len(), 1);
}

#[test]
fn a_connection_without_a_key_header_needs_no_key() {
    let directory = TempDir::new();
    let key = DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap();
    let db = user_db::create(directory.path(), 1, &key).unwrap();

    let without = Connection {
        headers: String::new(),
        ..with_key_header()
    };
    connections_create::connection(&db, &without).unwrap();

    assert!(prepare(&db, None, "Hello").unwrap().model.api_key.is_none());
}

#[test]
fn refused_messages_store_nothing() {
    let directory = TempDir::new();
    let db = database(&directory, false);
    assert!(matches!(prepare(&db, None, "Hello"), Err(SendError::NoKey(name)) if name == "Local"));

    let empty = TempDir::new();
    let key = DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap();
    let without_connection = user_db::create(empty.path(), 1, &key).unwrap();
    assert!(matches!(prepare(&without_connection, None, "Hello"), Err(SendError::NoConnection)));

    let other = TempDir::new();
    let db = database(&other, true);
    assert!(matches!(prepare(&db, None, "   "), Err(SendError::Empty)));
    assert!(matches!(prepare(&db, None, &"x".repeat(MAX_MESSAGE + 1)), Err(SendError::TooLong)));
    assert!(matches!(prepare(&db, Some(99), "Hello"), Err(SendError::NoSuchChat)));
    assert!(chats::list(&db).unwrap().is_empty());
}

#[test]
fn a_connection_changed_by_hand_is_not_used() {
    let directory = TempDir::new();
    let db = database(&directory, true);

    db.connection_for_tests()
        .execute("UPDATE ai_connections SET url = 'http://192.168.1.10/x'", [])
        .unwrap();

    assert!(matches!(prepare(&db, None, "Hello"), Err(SendError::BrokenConnection(..))));
    assert!(chats::list(&db).unwrap().is_empty());
}
