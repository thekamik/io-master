//! The chats (`Chat`) and their messages (`Message`), in the tables
//! `chats` and `chat_messages` of the database of the user: create, read,
//! update; never deleted (user, 2026-10-28). A message is never changed:
//! it is added to its chat ([`create_message`]); updating a chat writes
//! its title, its agent and its work unit.
//!
//! ```text
//! chats           id, title, agent, work_unit
//! chat_messages   (chat, position, speaker, text, time)
//! ```

use rusqlite::{Connection, Row, named_params};

use crate::db::error::DbError;
use crate::db::storage::user_database::UserDatabase;
use crate::model::ai::chat::{Chat, Message, Speaker};

/// The most chats of the user, and messages of one chat.
pub const MAX_CHATS: usize = 500;
pub const MAX_MESSAGES: usize = 500;

/// The tables, made when they are missing (`schema.rs`, `USER_TABLES`).
/// A work unit taken out of its plan leaves
/// its chats about nothing in particular.
pub const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS chats (
        id        INTEGER PRIMARY KEY,
        title     TEXT NOT NULL,
        agent     INTEGER REFERENCES ai_agents(id),
        work_unit INTEGER REFERENCES work_units(id) ON DELETE SET NULL
    );

    CREATE TABLE IF NOT EXISTS chat_messages (
        chat     INTEGER NOT NULL REFERENCES chats(id),
        position INTEGER NOT NULL,
        speaker  TEXT NOT NULL CHECK (speaker IN ('user', 'ai')),
        text     TEXT NOT NULL,
        time     INTEGER NOT NULL,
        PRIMARY KEY (chat, position)
    );
";

/// The name of a speaker in the database.
fn speaker_name(speaker: Speaker) -> &'static str {
    match speaker {
        Speaker::User => "user",
        Speaker::Ai => "ai",
    }
}

/// The speaker of its name; a name the program never writes is
/// `DbError::Corrupt`.
fn speaker_of(name: &str) -> Result<Speaker, DbError> {
    match name {
        "user" => Ok(Speaker::User),
        "ai" => Ok(Speaker::Ai),
        _ => Err(DbError::Corrupt { table: "chat_messages", column: "speaker", value: name.to_owned() }),
    }
}

/// Adds `message` as the last of the chat `chat`, inside the transaction
/// of its caller.
///
/// Called by: [`create_chat`], [`create_message`].
fn insert_message(connection: &Connection, chat: i64, message: &Message) -> Result<(), DbError> {
    let count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM chat_messages WHERE chat = :chat",
        named_params! { ":chat": chat },
        |row| row.get(0),
    )?;

    ensure!(
        usize::try_from(count).map_err(|error| DbError::Inconsistent(error.to_string()))? < MAX_MESSAGES,
        DbError::Inconsistent(format!("more than {MAX_MESSAGES} messages in a chat"))
    );

    connection.execute(
        "INSERT INTO chat_messages (chat, position, speaker, text, time) VALUES (:chat, :position, :speaker, :text, :time)",
        named_params! {
            ":chat": chat,
            ":position": count,
            ":speaker": speaker_name(message.speaker),
            ":text": message.text,
            ":time": message.time,
        },
    )?;

    Ok(())
}

/// Stores a new chat with its first messages, in one transaction, and gives
/// its id. A new object has no id yet (`id` 0): the database gives it.
///
/// Called by: `panels/logic/chat/chats.rs` (`new_chat`).
/// Calls: [`insert_message`].
pub fn create_chat(db: &UserDatabase, chat: &Chat) -> Result<i64, DbError> {
    ensure!(chat.id == 0, DbError::Inconsistent("a new chat has an id already".to_owned()));
    ensure!(chat.messages.len() <= MAX_MESSAGES, DbError::Inconsistent(format!("more than {MAX_MESSAGES} messages")));

    let transaction = db.connection().unchecked_transaction()?;
    let count: i64 = transaction.query_row("SELECT COUNT(*) FROM chats", [], |row| row.get(0))?;

    ensure!(
        usize::try_from(count).map_err(|error| DbError::Inconsistent(error.to_string()))? < MAX_CHATS,
        DbError::Inconsistent(format!("more than {MAX_CHATS} chats"))
    );

    transaction.execute(
        "INSERT INTO chats (title, agent, work_unit) VALUES (:title, :agent, :work_unit)",
        named_params! { ":title": chat.title, ":agent": chat.agent, ":work_unit": chat.work_unit },
    )?;

    let id = transaction.last_insert_rowid();

    for message in &chat.messages {
        insert_message(&transaction, id, message)?;
    }

    transaction.commit()?;

    Ok(id)
}

/// Adds `message` as the last of the chat `chat`.
///
/// Called by: `panels/logic/chat/chats.rs` (`add_question`, `add_answer`).
/// Calls: [`insert_message`].
pub fn create_message(db: &UserDatabase, chat: i64, message: &Message) -> Result<(), DbError> {
    ensure!(!message.text.is_empty(), DbError::Inconsistent("an empty message".to_owned()));

    insert_message(db.connection(), chat, message)
}

/// Reads the messages of the chat `id`, from the first.
///
/// Called by: [`read_chat`].
fn read_messages(connection: &Connection, id: i64) -> Result<Vec<Message>, DbError> {
    let mut statement =
        connection.prepare("SELECT speaker, text, time FROM chat_messages WHERE chat = :id ORDER BY position")?;
    let mut rows = statement.query(named_params! { ":id": id })?;
    let mut messages = Vec::new();

    for _ in 0..=MAX_MESSAGES {
        let Some(row) = rows.next()? else {
            return Ok(messages);
        };
        let speaker: String = row.get("speaker")?;

        messages.push(Message { speaker: speaker_of(&speaker)?, text: row.get("text")?, time: row.get("time")? });
    }

    Err(DbError::Inconsistent(format!("more than {MAX_MESSAGES} messages of a chat in the database")))
}

/// The model of a row of `chats`, its messages still empty.
///
/// Called by: [`read_chat`].
fn chat_of(row: &Row<'_>) -> Result<Chat, DbError> {
    Ok(Chat {
        id: row.get("id")?,
        title: row.get("title")?,
        agent: row.get("agent")?,
        work_unit: row.get("work_unit")?,
        messages: Vec::new(),
    })
}

/// Reads the chat `id` with its messages.
///
/// Called by: [`read_chats`]; `panels/logic/chat/chats.rs` (`rename_chat`,
/// `chat_question`).
/// Calls: [`chat_of`], [`read_messages`].
pub fn read_chat(db: &UserDatabase, id: i64) -> Result<Chat, DbError> {
    let connection = db.connection();
    let mut statement = connection.prepare("SELECT id, title, agent, work_unit FROM chats WHERE id = :id")?;
    let mut rows = statement.query(named_params! { ":id": id })?;
    let Some(row) = rows.next()? else {
        return Err(DbError::NoRow { table: "chats", id });
    };
    let mut chat = chat_of(row)?;

    chat.messages = read_messages(connection, id)?;

    Ok(chat)
}

/// Reads every chat with its messages, the newest first.
///
/// Called by: nothing yet (the views rewritten on `db/`).
/// Calls: [`read_chat`].
pub fn read_chats(db: &UserDatabase) -> Result<Vec<Chat>, DbError> {
    let mut statement = db.connection().prepare("SELECT id FROM chats ORDER BY id DESC")?;
    let mut rows = statement.query([])?;
    let mut chats = Vec::new();

    for _ in 0..=MAX_CHATS {
        let Some(row) = rows.next()? else {
            return Ok(chats);
        };

        chats.push(read_chat(db, row.get("id")?)?);
    }

    Err(DbError::Inconsistent(format!("more than {MAX_CHATS} chats in the database")))
}

/// Writes the title, the agent and the work unit of the chat `chat.id`
/// anew; its messages stay as they were (a message is never changed).
///
/// Called by: `panels/logic/chat/chats.rs` (`rename_chat`).
pub fn update_chat(db: &UserDatabase, chat: &Chat) -> Result<(), DbError> {
    let changed = db.connection().execute(
        "UPDATE chats SET title = :title, agent = :agent, work_unit = :work_unit WHERE id = :id",
        named_params! { ":id": chat.id, ":title": chat.title, ":agent": chat.agent, ":work_unit": chat.work_unit },
    )?;

    ensure!(changed == 1, DbError::NoRow { table: "chats", id: chat.id });

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Tests of the chats: stored with their messages, a message added, the
    //! title changed while the messages stay.

    use super::*;

    use crate::test_support::database::database;

    fn message(speaker: Speaker, text: &str) -> Message {
        Message { speaker, text: text.to_owned(), time: 1_793_000_000 }
    }

    #[test]
    fn a_chat_is_stored_a_message_added_and_its_title_changed() {
        let (_directory, db) = database();
        let new = Chat {
            id: 0,
            title: "ADV_ARC.1-1".to_owned(),
            agent: None,
            work_unit: None,
            messages: vec![message(Speaker::User, "What does the TSF protect?")],
        };
        let id = create_chat(&db, &new).unwrap();

        create_message(&db, id, &message(Speaker::Ai, "The keys.")).unwrap();

        let mut stored = read_chat(&db, id).unwrap();

        assert_eq!(stored.messages.len(), 2);
        assert_eq!(stored.messages[1], message(Speaker::Ai, "The keys."));

        stored.title = "The keys".to_owned();
        stored.messages.clear();
        update_chat(&db, &stored).unwrap();

        let read = read_chats(&db).unwrap();

        assert_eq!(read[0].title, "The keys");
        assert_eq!(read[0].messages.len(), 2);
    }

    #[test]
    fn an_empty_message_and_a_missing_chat_are_refused() {
        let (_directory, db) = database();

        assert!(matches!(create_message(&db, 1, &message(Speaker::User, "")), Err(DbError::Inconsistent(_))));
        assert!(matches!(create_message(&db, 1, &message(Speaker::User, "x")), Err(DbError::Sql(_))));
        assert!(matches!(read_chat(&db, 1), Err(DbError::NoRow { .. })));
    }
}
