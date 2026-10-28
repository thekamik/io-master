//! What a chat keeps (`db/ai/chats/`, tables `chats` and `messages`, in
//! the encrypted database of the evaluator): a new chat with the title
//! taken from its first message, and a reply of the model with how it
//! ended ([`Ending`]), so that the page shows the same history after a
//! reload:
//!
//! ```text
//! complete   the model finished
//! cut        the reply reached the largest length and was cut
//! stopped    the evaluator stopped it; the text up to then is kept
//! refused    the model declined; any partial text is dropped (it
//!            may be the beginning of what was declined), and only
//!            the fact is kept
//! failed     the request failed; the explanation is kept, and the
//!            text written before the failure
//! ```
//!
//! Times are kept as seconds since 1970 (UTC); SQLite turns them into
//! the local time of the computer when they are read. What of the history
//! goes to the model is decided by `ai/tasks/chat.rs`.

use crate::ai::api::completions::{Reply, ReplyEnd, StopReason};
use crate::db_old::ai::chats as db_chats;
use crate::db_old::error::DbError;
use crate::db_old::user::database::UserDb;
use crate::model_old::chats::Ending;

/// The longest title of a chat, in characters; the title is the
/// beginning of the first message.
const TITLE_LENGTH: usize = 60;

/// The title of a new chat: the beginning of its first message, on one
/// line, at most [`TITLE_LENGTH`] characters.
fn title_of(text: &str) -> String {
    let line = text.split_whitespace().collect::<Vec<_>>().join(" ");

    if line.chars().count() <= TITLE_LENGTH {
        return line;
    }

    let mut title = String::with_capacity(TITLE_LENGTH * 4);

    for character in line.chars().take(TITLE_LENGTH - 1) {
        title.push(character);
    }

    title.push('…');
    title
}

/// Starts a chat with the first message of the evaluator; returns the
/// number of the chat.
pub fn start(db: &UserDb, first_message: &str) -> Result<i64, DbError> {
    let id = db_chats::create::chat(db, &title_of(first_message))?;
    db_chats::create::message(db, id, first_message)?;

    Ok(id)
}

/// How a reply ended, as it is kept: the ending, and for a failed reply
/// the explanation.
pub fn ending_of(reply: &Reply) -> (Ending, Option<String>) {
    match &reply.end {
        ReplyEnd::Finished(StopReason::Cut) => (Ending::Cut, None),
        ReplyEnd::Finished(StopReason::Refused) => (Ending::Refused, None),
        ReplyEnd::Finished(StopReason::Complete) => (Ending::Complete, None),
        ReplyEnd::Cancelled => (Ending::Stopped, None),
        ReplyEnd::Failed(error) => (Ending::Failed, Some(error.to_string())),
    }
}

/// Adds a reply of the model to chat `chat_id`, with how it ended.
pub fn add_reply(db: &UserDb, chat_id: i64, reply: &Reply) -> Result<(), DbError> {
    let (ending, note) = ending_of(reply);

    // A refused reply is dropped (see the top of this file).
    let text = if ending == Ending::Refused { "" } else { reply.text.as_str() };

    db_chats::create::reply(
        db,
        chat_id,
        &db_chats::create::Reply {
            text,
            model: reply.model.as_deref(),
            ending: &ending,
            note: note.as_deref(),
            input_tokens: reply.input_tokens,
            output_tokens: reply.output_tokens,
        },
    )
}

#[cfg(test)]
mod tests {
    //! Unit tests of `stored.rs`.

    use super::*;

    use crate::ai::api::completions::AiError;
    use crate::db::encryption::database_key::DatabaseKey;
    use crate::db_old::user::database as user_db;
    use crate::model_old::ai::Speaker;
    use crate::security::random::fill_random;
    use crate::test_support::TempDir;

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
        db_chats::create::message(&db, id, "And ADV_TDS?").unwrap();

        let messages = db_chats::read::messages(&db, id).unwrap();

        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].speaker, Speaker::Evaluator);
        assert_eq!(messages[0].text, "What is ADV_FSP?");
        assert_eq!(messages[1].speaker, Speaker::Model);
        assert_eq!(messages[1].model.as_deref(), Some("local-model"));
        assert_eq!(messages[1].ending, Ending::Complete);
        assert_eq!(messages[2].text, "And ADV_TDS?");

        // "2026-10-14 15:04"
        assert_eq!(messages[0].created.len(), 16);

        assert_eq!(db_chats::read::chat_title(&db, id).unwrap().as_deref(), Some("What is ADV_FSP?"));
    }

    #[test]
    fn the_newest_chat_comes_first() {
        let directory = TempDir::new();
        let db = database(&directory);

        let first = start(&db, "first").unwrap();
        let second = start(&db, "second").unwrap();

        let ids: Vec<i64> = db_chats::read::chats(&db).unwrap().iter().map(|chat| chat.id).collect();
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

        let messages = db_chats::read::messages(&db, id).unwrap();
        let endings: Vec<&Ending> = messages[1..].iter().map(|message| &message.ending).collect();

        assert_eq!(endings, [&Ending::Stopped, &Ending::Cut, &Ending::Failed, &Ending::Refused]);
        assert_eq!(messages[3].text, "partial");
        assert_eq!(
            messages[3].note.as_deref(),
            Some(AiError::ServiceFailed(503, "overloaded".to_owned()).to_string().as_str())
        );

        // The text of a refused reply is not kept, and not sent again.
        assert_eq!(messages[4].text, "");
        assert_eq!(crate::ai::tasks::chat::turns(&messages).len(), 4);
    }
}
