//! Deleting chats.

use crate::db_old::error::DbError;
use crate::db_old::user::database::UserDb;

/// Deletes a chat with its messages; returns whether it existed.
pub fn chat(db: &UserDb, chat_id: i64) -> Result<bool, DbError> {
    let connection = db.connection();

    // The text of the chat is overwritten in the file, not only
    // marked as free (see UserDb::remove_secret).
    connection.execute_batch("PRAGMA secure_delete = ON")?;
    connection.execute("DELETE FROM messages WHERE chat_id = ?1", [chat_id])?;

    Ok(connection.execute("DELETE FROM chats WHERE id = ?1", [chat_id])? > 0)
}
