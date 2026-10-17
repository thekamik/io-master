//! Unit tests of `chats.rs`.

use super::*;

use crate::model::ai::Speaker;

use crate::ai::api::completions::AiError;
use crate::db::encryption::database_key::DatabaseKey;
use crate::db::user::database as user_db;
use crate::random::random::fill_random;
use crate::tpm::test_support::TempDir;

fn database(directory: &TempDir) -> UserDb {
    let key = DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap();

    user_db::create(directory.path(), 1, &key).unwrap()
}

fn reply(text: &str, end: ReplyEnd) -> Reply {
    Reply { text: text.to_owned(), model: Some("local-model".to_owned()), input_tokens: 10, output_tokens: 5, end }
}

#[test]
fn a_chat_keeps_its_messages_in_order() {
    let directory = TempDir::new();
    let db = database(&directory);

    let id = start(&db, "What is ADV_FSP?").unwrap();
    add_reply(&db, id, &reply("The functional specification.", ReplyEnd::Finished(StopReason::Complete))).unwrap();
    add_message(&db, id, "And ADV_TDS?").unwrap();

    let messages = messages(&db, id).unwrap();

    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0].speaker, Speaker::Evaluator);
    assert_eq!(messages[0].text, "What is ADV_FSP?");
    assert_eq!(messages[1].speaker, Speaker::Model);
    assert_eq!(messages[1].model.as_deref(), Some("local-model"));
    assert_eq!(messages[1].ending, Ending::Complete);
    assert_eq!(messages[2].text, "And ADV_TDS?");

    // "2026-10-14 15:04"
    assert_eq!(messages[0].created.len(), 16);

    assert_eq!(title(&db, id).unwrap().as_deref(), Some("What is ADV_FSP?"));
}

#[test]
fn the_newest_chat_comes_first() {
    let directory = TempDir::new();
    let db = database(&directory);

    let first = start(&db, "first").unwrap();
    let second = start(&db, "second").unwrap();

    let ids: Vec<i64> = list(&db).unwrap().iter().map(|chat| chat.id).collect();
    assert_eq!(ids, [second, first]);
}

#[test]
fn a_long_first_message_gives_a_short_title_on_one_line() {
    assert_eq!(title_of("  two\nlines  "), "two lines");

    let title = title_of(&"ż".repeat(100));
    assert_eq!(title.chars().count(), TITLE_LENGTH);
    assert!(title.ends_with('…'));
}

#[test]
fn unfinished_replies_are_kept_with_their_ending() {
    let directory = TempDir::new();
    let db = database(&directory);
    let id = start(&db, "question").unwrap();

    add_reply(&db, id, &reply("half", ReplyEnd::Cancelled)).unwrap();
    add_reply(&db, id, &reply("long", ReplyEnd::Finished(StopReason::Cut))).unwrap();
    add_reply(&db, id, &reply("partial", ReplyEnd::Failed(AiError::ServiceFailed(503, "overloaded".to_owned()))))
        .unwrap();
    add_reply(&db, id, &reply("dropped", ReplyEnd::Finished(StopReason::Refused))).unwrap();

    let messages = messages(&db, id).unwrap();
    let endings: Vec<&Ending> = messages[1..].iter().map(|message| &message.ending).collect();

    assert_eq!(endings, [&Ending::Stopped, &Ending::Cut, &Ending::Failed, &Ending::Refused]);
    assert_eq!(messages[3].text, "partial");
    assert_eq!(
        messages[3].note.as_deref(),
        Some(AiError::ServiceFailed(503, "overloaded".to_owned()).to_string().as_str())
    );

    // The text of a refused reply is not kept, and not sent again.
    assert_eq!(messages[4].text, "");
    assert_eq!(crate::ai::chat::memory::turns(&messages).len(), 4);
}

#[test]
fn a_deleted_chat_is_gone_with_its_messages() {
    let directory = TempDir::new();
    let db = database(&directory);

    let kept = start(&db, "kept").unwrap();
    let deleted = start(&db, "deleted").unwrap();

    assert!(delete(&db, deleted).unwrap());
    assert!(!delete(&db, deleted).unwrap());

    assert_eq!(title(&db, deleted).unwrap(), None);
    assert!(messages(&db, deleted).unwrap().is_empty());
    assert_eq!(messages(&db, kept).unwrap().len(), 1);
}
