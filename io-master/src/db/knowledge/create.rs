//! Storing knowledge bases and their sources.

use rusqlite::named_params;

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model::knowledge::{AgentFields, MAX_AGENTS, MAX_BASES, MAX_PASSAGES, MAX_SOURCES, Passage, SourceKind};
use crate::model::time::now;

/// A limit as a number of SQL.
fn limit(max: usize) -> Result<i64, DbError> {
    i64::try_from(max).map_err(|error| DbError::Inconsistent(error.to_string()))
}

/// Stores a new base named `name`; gives its id. Refused when a base has
/// that name already or there are [`MAX_BASES`] already. One transaction.
///
/// Called by: `knowledge_list_create` (`views/knowledge/knowledge.rs`).
pub fn base(db: &UserDb, name: &str, description: &str) -> Result<i64, DbError> {
    let transaction = db.connection().unchecked_transaction()?;
    let count: i64 = transaction.query_row("SELECT COUNT(*) FROM knowledge_bases", [], |row| row.get(0))?;
    let taken: bool =
        transaction.query_row("SELECT COUNT(*) > 0 FROM knowledge_bases WHERE name = ?1", [name], |row| row.get(0))?;

    if count >= limit(MAX_BASES)? {
        return Err(DbError::Inconsistent(format!("an evaluator keeps at most {MAX_BASES} knowledge bases")));
    }

    if taken {
        return Err(DbError::Inconsistent("a knowledge base of this name exists already".to_owned()));
    }

    transaction.execute(
        "INSERT INTO knowledge_bases (name, description, created) VALUES (:name, :description, :created)",
        named_params! { ":name": name, ":description": description, ":created": now() },
    )?;
    let id = transaction.last_insert_rowid();
    transaction.commit()?;

    Ok(id)
}

/// Stores a source of base `base` with its passages; gives its id. Refused
/// when the base does not exist, has [`MAX_SOURCES`] already, or the source
/// has more than [`MAX_PASSAGES`] passages. One transaction.
///
/// Called by: `views/knowledge/knowledge_base/add.rs`.
pub fn source(db: &UserDb, base: i64, kind: SourceKind, name: &str, passages: &[Passage]) -> Result<i64, DbError> {
    if passages.len() > MAX_PASSAGES {
        return Err(DbError::Inconsistent(format!("a source has at most {MAX_PASSAGES} passages")));
    }

    let transaction = db.connection().unchecked_transaction()?;
    let exists: bool =
        transaction.query_row("SELECT COUNT(*) > 0 FROM knowledge_bases WHERE id = ?1", [base], |row| row.get(0))?;
    let count: i64 =
        transaction.query_row("SELECT COUNT(*) FROM knowledge_sources WHERE base_id = ?1", [base], |row| row.get(0))?;

    if !exists {
        return Err(DbError::NoRow { table: "knowledge_bases", id: base });
    }

    if count >= limit(MAX_SOURCES)? {
        return Err(DbError::Inconsistent(format!("a knowledge base has at most {MAX_SOURCES} sources")));
    }

    transaction.execute(
        "INSERT INTO knowledge_sources (base_id, kind, name, passages, added)
         VALUES (:base, :kind, :name, :passages, :added)",
        named_params! {
            ":base": base, ":kind": kind.name(), ":name": name, ":passages": limit(passages.len())?, ":added": now(),
        },
    )?;
    let id = transaction.last_insert_rowid();

    for passage in passages {
        transaction.execute(
            "INSERT INTO knowledge_passages (text, source_id, page, label) VALUES (:text, :source, :page, :label)",
            named_params! { ":text": passage.text, ":source": id, ":page": passage.page, ":label": passage.label },
        )?;
    }

    transaction.commit()?;

    Ok(id)
}

/// Stores a new agent of `fields` with the bases it draws on; gives its id.
/// Refused when an agent has that name already, there are [`MAX_AGENTS`]
/// already, or a base does not exist. One transaction.
///
/// Called by: `agent_list_create` (`views/knowledge/knowledge.rs`).
/// Calls: `update::bases_of`.
pub fn agent(db: &UserDb, fields: &AgentFields) -> Result<i64, DbError> {
    let transaction = db.connection().unchecked_transaction()?;
    let count: i64 = transaction.query_row("SELECT COUNT(*) FROM knowledge_agents", [], |row| row.get(0))?;
    let taken: bool =
        transaction
            .query_row("SELECT COUNT(*) > 0 FROM knowledge_agents WHERE name = ?1", [&fields.name], |row| row.get(0))?;

    if count >= limit(MAX_AGENTS)? {
        return Err(DbError::Inconsistent(format!("an evaluator keeps at most {MAX_AGENTS} agents")));
    }

    if taken {
        return Err(DbError::Inconsistent("an agent of this name exists already".to_owned()));
    }

    transaction.execute(
        "INSERT INTO knowledge_agents (name, instructions, glossary, passages, characters, created)
         VALUES (:name, :instructions, :glossary, :passages, :characters, :created)",
        named_params! {
            ":name": fields.name,
            ":instructions": fields.instructions,
            ":glossary": fields.glossary,
            ":passages": limit(fields.passages)?,
            ":characters": limit(fields.characters)?,
            ":created": now(),
        },
    )?;
    let id = transaction.last_insert_rowid();
    super::update::bases_of(&transaction, id, &fields.bases)?;
    transaction.commit()?;

    Ok(id)
}
