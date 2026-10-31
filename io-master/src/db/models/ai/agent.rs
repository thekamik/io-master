//! The AI agents (`Agent`), in the table `ai_agents` of the database of the
//! user, with their lists in tables of their own: create, read, update;
//! never deleted (user, 2026-10-28).
//!
//! ```text
//! ai_agents                 id, name (unique), integration, instructions, max_passages, max_characters
//! ai_agent_knowledge_bases  (agent, position, knowledge_base)   knowledge_bases
//! ai_agent_glossary         (agent, position, term, meaning)    glossary
//! ```

use rusqlite::{Connection, Row, named_params};

use crate::db::error::DbError;
use crate::db::models::ai::rag::{MAX_BASES, MAX_FOUND};
use crate::db::models::lists::{check_length, position, read_ids, write_ids};
use crate::db::storage::user_database::UserDatabase;
use crate::model::ai::agent::{Agent, GlossaryEntry};

/// The most agents of the user, and terms of the glossary of one agent.
pub const MAX_AGENTS: usize = 50;
pub const MAX_TERMS: usize = 500;

/// The most characters of knowledge an agent gives with one question.
pub const MAX_CHARACTERS: u32 = 40_000;

/// The tables, made when they are missing (`schema.rs`, `USER_TABLES`), after the
/// integrations and the knowledge bases.
pub const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS ai_agents (
        id             INTEGER PRIMARY KEY,
        name           TEXT NOT NULL UNIQUE,
        integration    INTEGER NOT NULL REFERENCES ai_integrations(id),
        instructions   TEXT NOT NULL,
        max_passages   INTEGER NOT NULL,
        max_characters INTEGER NOT NULL
    );

    CREATE TABLE IF NOT EXISTS ai_agent_knowledge_bases (
        agent          INTEGER NOT NULL REFERENCES ai_agents(id),
        position       INTEGER NOT NULL,
        knowledge_base INTEGER NOT NULL REFERENCES knowledge_bases(id),
        PRIMARY KEY (agent, position),
        UNIQUE (agent, knowledge_base)
    );

    CREATE TABLE IF NOT EXISTS ai_agent_glossary (
        agent    INTEGER NOT NULL REFERENCES ai_agents(id),
        position INTEGER NOT NULL,
        term     TEXT NOT NULL,
        meaning  TEXT NOT NULL,
        PRIMARY KEY (agent, position)
    );
";

/// The columns every read of an agent selects.
const COLUMNS: &str = "id, name, integration, instructions, max_passages, max_characters";

/// Refuses an agent without a name, with lists or limits over theirs, or a
/// term of its glossary without a meaning.
///
/// Called by: [`create_agent`], [`update_agent`].
fn check_agent(agent: &Agent) -> Result<(), DbError> {
    check_length(agent.knowledge_bases.len(), MAX_BASES, "knowledge bases of an agent")?;
    check_length(agent.glossary.len(), MAX_TERMS, "terms of a glossary")?;

    ensure!(!agent.name.is_empty(), DbError::Inconsistent("an agent without a name".to_owned()));
    ensure!(
        usize::try_from(agent.max_passages).map_err(|error| DbError::Inconsistent(error.to_string()))? <= MAX_FOUND,
        DbError::Inconsistent(format!("an agent that gives more than {MAX_FOUND} passages"))
    );
    ensure!(
        agent.max_characters <= MAX_CHARACTERS,
        DbError::Inconsistent(format!("an agent that gives more than {MAX_CHARACTERS} characters"))
    );

    for entry in &agent.glossary {
        ensure!(
            !entry.term.is_empty() && !entry.meaning.is_empty(),
            DbError::Inconsistent("a term of a glossary without its meaning".to_owned())
        );
    }

    Ok(())
}

/// Writes the lists of the agent `id` anew: its knowledge bases and its
/// glossary. Run inside the transaction of its caller.
///
/// Called by: [`create_agent`], [`update_agent`].
/// Calls: `lists::write_ids`.
fn write_lists(connection: &Connection, id: i64, agent: &Agent) -> Result<(), DbError> {
    write_ids(
        connection,
        "DELETE FROM ai_agent_knowledge_bases WHERE agent = :owner",
        "INSERT INTO ai_agent_knowledge_bases (agent, position, knowledge_base) VALUES (:owner, :position, :id)",
        id,
        &agent.knowledge_bases,
    )?;

    connection.execute("DELETE FROM ai_agent_glossary WHERE agent = :id", named_params! { ":id": id })?;

    for (index, entry) in agent.glossary.iter().enumerate() {
        connection.execute(
            "INSERT INTO ai_agent_glossary (agent, position, term, meaning) VALUES (:id, :position, :term, :meaning)",
            named_params! { ":id": id, ":position": position(index)?, ":term": entry.term, ":meaning": entry.meaning },
        )?;
    }

    Ok(())
}

/// Stores a new agent and gives its id, in one transaction with its lists.
/// A new object has no id yet (`id` 0): the database gives it.
///
/// Called by: nothing yet (the views rewritten on `db/`).
/// Calls: [`check_agent`], [`write_lists`].
pub fn create_agent(db: &UserDatabase, agent: &Agent) -> Result<i64, DbError> {
    ensure!(agent.id == 0, DbError::Inconsistent("a new agent has an id already".to_owned()));
    check_agent(agent)?;

    let transaction = db.connection().unchecked_transaction()?;
    let count: i64 = transaction.query_row("SELECT COUNT(*) FROM ai_agents", [], |row| row.get(0))?;

    ensure!(
        usize::try_from(count).map_err(|error| DbError::Inconsistent(error.to_string()))? < MAX_AGENTS,
        DbError::Inconsistent(format!("more than {MAX_AGENTS} agents"))
    );

    transaction.execute(
        "INSERT INTO ai_agents (name, integration, instructions, max_passages, max_characters)
         VALUES (:name, :integration, :instructions, :max_passages, :max_characters)",
        named_params! {
            ":name": agent.name,
            ":integration": agent.integration,
            ":instructions": agent.instructions,
            ":max_passages": agent.max_passages,
            ":max_characters": agent.max_characters,
        },
    )?;

    let id = transaction.last_insert_rowid();

    write_lists(&transaction, id, agent)?;
    transaction.commit()?;

    Ok(id)
}

/// The model of a row of `ai_agents`, its lists still empty.
///
/// Called by: [`read_agent`].
fn agent_of(row: &Row<'_>) -> Result<Agent, DbError> {
    Ok(Agent {
        id: row.get("id")?,
        name: row.get("name")?,
        integration: row.get("integration")?,
        instructions: row.get("instructions")?,
        knowledge_bases: Vec::new(),
        glossary: Vec::new(),
        max_passages: row.get("max_passages")?,
        max_characters: row.get("max_characters")?,
    })
}

/// Reads the glossary of the agent `id`, in its order.
///
/// Called by: [`read_agent`].
fn read_glossary(connection: &Connection, id: i64) -> Result<Vec<GlossaryEntry>, DbError> {
    let mut statement =
        connection.prepare("SELECT term, meaning FROM ai_agent_glossary WHERE agent = :id ORDER BY position")?;
    let mut rows = statement.query(named_params! { ":id": id })?;
    let mut glossary = Vec::new();

    for _ in 0..=MAX_TERMS {
        let Some(row) = rows.next()? else {
            return Ok(glossary);
        };

        glossary.push(GlossaryEntry { term: row.get("term")?, meaning: row.get("meaning")? });
    }

    Err(DbError::Inconsistent(format!("more than {MAX_TERMS} terms of a glossary in the database")))
}

/// Reads the agent `id` with its lists.
///
/// Called by: [`read_agents`]; the views rewritten on `db/`.
/// Calls: [`agent_of`], `lists::read_ids`, [`read_glossary`].
pub fn read_agent(db: &UserDatabase, id: i64) -> Result<Agent, DbError> {
    let connection = db.connection();
    let mut statement = connection.prepare(&format!("SELECT {COLUMNS} FROM ai_agents WHERE id = :id"))?;
    let mut rows = statement.query(named_params! { ":id": id })?;
    let Some(row) = rows.next()? else {
        return Err(DbError::NoRow { table: "ai_agents", id });
    };
    let mut agent = agent_of(row)?;

    agent.knowledge_bases = read_ids(
        connection,
        "SELECT knowledge_base AS id FROM ai_agent_knowledge_bases WHERE agent = :owner ORDER BY position",
        id,
        MAX_BASES,
    )?;
    agent.glossary = read_glossary(connection, id)?;

    Ok(agent)
}

/// Reads every agent, by its name.
///
/// Called by: nothing yet (the views rewritten on `db/`).
/// Calls: [`read_agent`].
pub fn read_agents(db: &UserDatabase) -> Result<Vec<Agent>, DbError> {
    let mut statement = db.connection().prepare("SELECT id FROM ai_agents ORDER BY name")?;
    let mut rows = statement.query([])?;
    let mut agents = Vec::new();

    for _ in 0..=MAX_AGENTS {
        let Some(row) = rows.next()? else {
            return Ok(agents);
        };

        agents.push(read_agent(db, row.get("id")?)?);
    }

    Err(DbError::Inconsistent(format!("more than {MAX_AGENTS} agents in the database")))
}

/// Writes the agent `agent.id` anew, with its lists, in one transaction.
///
/// Called by: nothing yet (the views rewritten on `db/`).
/// Calls: [`check_agent`], [`write_lists`].
pub fn update_agent(db: &UserDatabase, agent: &Agent) -> Result<(), DbError> {
    check_agent(agent)?;

    let transaction = db.connection().unchecked_transaction()?;
    let changed = transaction.execute(
        "UPDATE ai_agents SET name = :name, integration = :integration, instructions = :instructions,
                max_passages = :max_passages, max_characters = :max_characters
         WHERE id = :id",
        named_params! {
            ":id": agent.id,
            ":name": agent.name,
            ":integration": agent.integration,
            ":instructions": agent.instructions,
            ":max_passages": agent.max_passages,
            ":max_characters": agent.max_characters,
        },
    )?;

    ensure!(changed == 1, DbError::NoRow { table: "ai_agents", id: agent.id });

    write_lists(&transaction, agent.id, agent)?;
    transaction.commit()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Tests of the agents: stored with their lists, read, changed, refused.

    use super::*;

    use crate::db::models::ai::integration::create_integration;
    use crate::db::models::ai::rag::create_knowledge_base;
    use crate::model::ai::rag::KnowledgeBase;
    use crate::test_support::database::{agent, database, integration};

    #[test]
    fn an_agent_is_stored_with_its_lists_read_and_changed() {
        let (_directory, db) = database();
        let integration = create_integration(&db, &integration()).unwrap();
        let base = KnowledgeBase { id: 0, name: "CEM".to_owned(), description: String::new() };
        let new = Agent {
            knowledge_bases: vec![create_knowledge_base(&db, &base).unwrap()],
            glossary: vec![GlossaryEntry { term: "TSF".to_owned(), meaning: "TOE Security Functionality".to_owned() }],
            ..agent(integration)
        };
        let id = create_agent(&db, &new).unwrap();
        let mut stored = read_agent(&db, id).unwrap();

        assert_eq!(stored, Agent { id, ..new });

        stored.glossary.clear();
        stored.max_passages = 4;
        update_agent(&db, &stored).unwrap();

        assert_eq!(read_agents(&db).unwrap(), vec![stored]);
    }

    #[test]
    fn what_is_no_agent_is_refused() {
        let (_directory, db) = database();
        let integration = create_integration(&db, &integration()).unwrap();
        let empty_term = GlossaryEntry { term: "TSF".to_owned(), meaning: String::new() };

        assert!(matches!(
            create_agent(&db, &Agent { name: String::new(), ..agent(integration) }),
            Err(DbError::Inconsistent(_))
        ));
        assert!(matches!(
            create_agent(&db, &Agent { glossary: vec![empty_term], ..agent(integration) }),
            Err(DbError::Inconsistent(_))
        ));
        assert!(matches!(
            create_agent(&db, &Agent { max_passages: 51, ..agent(integration) }),
            Err(DbError::Inconsistent(_))
        ));
        assert!(matches!(create_agent(&db, &agent(integration + 1)), Err(DbError::Sql(_))));
        assert!(matches!(read_agent(&db, 3), Err(DbError::NoRow { .. })));
    }
}
