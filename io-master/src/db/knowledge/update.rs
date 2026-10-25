//! Changing a knowledge base.

use rusqlite::named_params;

use crate::db::error::DbError;
use crate::db::user::database::UserDb;

/// Gives base `id` the name `name` and the description `description`;
/// gives whether it exists. Refused when another base has that name.
///
/// Called by: `knowledge_base_save` (`views/knowledge/knowledge.rs`).
pub fn base(db: &UserDb, id: i64, name: &str, description: &str) -> Result<bool, DbError> {
    let transaction = db.connection().unchecked_transaction()?;
    let taken: bool = transaction.query_row(
        "SELECT COUNT(*) > 0 FROM knowledge_bases WHERE name = :name AND id != :id",
        named_params! { ":name": name, ":id": id },
        |row| row.get(0),
    )?;

    if taken {
        return Err(DbError::Inconsistent("a knowledge base of this name exists already".to_owned()));
    }

    let changed = transaction.execute(
        "UPDATE knowledge_bases SET name = :name, description = :description WHERE id = :id",
        named_params! { ":name": name, ":description": description, ":id": id },
    )?;
    transaction.commit()?;

    Ok(changed > 0)
}
