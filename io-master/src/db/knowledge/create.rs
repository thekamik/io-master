//! Storing knowledge bases, their sources and agents; an agent imported
//! with its bases in one transaction (step 57.7).

use rusqlite::{Transaction, named_params};

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model_old::knowledge::{
    AgentFields, MAX_AGENTS, MAX_BASES, MAX_PASSAGES, MAX_SOURCES, NAME_MAX, NewBase, Passage, SourceKind,
};
use crate::model_old::time::now;

/// The most names tried for an imported base or agent whose name is taken.
const MAX_NAME_TRIES: usize = 100;

/// A limit as a number of SQL.
fn limit(max: usize) -> Result<i64, DbError> {
    i64::try_from(max).map_err(|error| DbError::Inconsistent(error.to_string()))
}

/// Stores a new base named `name`; gives its id. Refused when a base has
/// that name already or there are [`MAX_BASES`] already. One transaction.
///
/// Called by: `knowledge_list_create` (`views/knowledge/knowledge.rs`).
/// Calls: [`insert_base`].
pub fn base(db: &UserDb, name: &str, description: &str) -> Result<i64, DbError> {
    let transaction = db.connection().unchecked_transaction()?;
    let id = insert_base(&transaction, name, description)?;
    transaction.commit()?;

    Ok(id)
}

/// Stores a source of base `base` with its passages; gives its id. Refused
/// when the base does not exist, has [`MAX_SOURCES`] already, or the source
/// has more than [`MAX_PASSAGES`] passages. One transaction.
///
/// Called by: `views/knowledge/knowledge_base/add.rs`.
/// Calls: [`insert_source`].
pub fn source(db: &UserDb, base: i64, kind: SourceKind, name: &str, passages: &[Passage]) -> Result<i64, DbError> {
    let transaction = db.connection().unchecked_transaction()?;
    let id = insert_source(&transaction, base, kind, name, passages)?;
    transaction.commit()?;

    Ok(id)
}

/// Stores a new agent of `fields` with the bases it draws on; gives its id.
/// Refused when an agent has that name already, there are [`MAX_AGENTS`]
/// already, or a base does not exist. One transaction.
///
/// Called by: `agent_list_create` (`views/knowledge/knowledge.rs`).
/// Calls: [`insert_agent`].
pub fn agent(db: &UserDb, fields: &AgentFields) -> Result<i64, DbError> {
    let transaction = db.connection().unchecked_transaction()?;
    let id = insert_agent(&transaction, fields)?;
    transaction.commit()?;

    Ok(id)
}

/// Stores an imported agent of `fields` with its `bases` (their sources and
/// passages), all or nothing in one transaction; a name taken already gets
/// " (imported)" (or " (imported N)"). The bases of `fields` are those
/// stored here. Gives the id of the agent.
///
/// Called by: `agent_list_import` (`views/knowledge/knowledge.rs`).
/// Calls: [`free_name`], [`insert_base`], [`insert_source`], [`insert_agent`].
pub fn imported_agent(db: &UserDb, fields: &AgentFields, bases: &[NewBase]) -> Result<i64, DbError> {
    let transaction = db.connection().unchecked_transaction()?;
    let mut ids = Vec::with_capacity(bases.len());

    for base in bases {
        let name = free_name(&transaction, "knowledge_bases", &base.name)?;
        let id = insert_base(&transaction, &name, &base.description)?;

        for source in &base.sources {
            insert_source(&transaction, id, source.kind, &source.name, &source.passages)?;
        }

        ids.push(id);
    }

    let stored =
        AgentFields { name: free_name(&transaction, "knowledge_agents", &fields.name)?, bases: ids, ..fields.clone() };
    let id = insert_agent(&transaction, &stored)?;
    transaction.commit()?;

    Ok(id)
}

/// `wanted`, or the first of `wanted (imported)`, `wanted (imported 2)`, ...
/// that no row of `table` (`knowledge_bases` or `knowledge_agents`) has;
/// `wanted` is shortened so that the name keeps within `NAME_MAX`.
///
/// Called by: [`imported_agent`].
fn free_name(transaction: &Transaction<'_>, table: &'static str, wanted: &str) -> Result<String, DbError> {
    // The table comes from the callers above, never from outside.
    let sql = format!("SELECT COUNT(*) > 0 FROM {table} WHERE name = ?1");

    for attempt in 0..MAX_NAME_TRIES {
        let suffix = match attempt {
            0 => String::new(),
            1 => " (imported)".to_owned(),
            _ => format!(" (imported {attempt})"),
        };
        let kept: String = wanted.chars().take(NAME_MAX.saturating_sub(suffix.chars().count())).collect();
        let name = format!("{kept}{suffix}");
        let taken: bool = transaction.query_row(&sql, [&name], |row| row.get(0))?;

        if !taken {
            return Ok(name);
        }
    }

    Err(DbError::Inconsistent(format!("no free name for \"{wanted}\"")))
}

/// Inserts a base, inside `transaction`; refused as [`base`] says.
///
/// Called by: [`base`], [`imported_agent`].
fn insert_base(transaction: &Transaction<'_>, name: &str, description: &str) -> Result<i64, DbError> {
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

    Ok(transaction.last_insert_rowid())
}

/// Inserts a source with its passages, inside `transaction`; refused as
/// [`source`] says.
///
/// Called by: [`source`], [`imported_agent`].
fn insert_source(
    transaction: &Transaction<'_>,
    base: i64,
    kind: SourceKind,
    name: &str,
    passages: &[Passage],
) -> Result<i64, DbError> {
    if passages.len() > MAX_PASSAGES {
        return Err(DbError::Inconsistent(format!("a source has at most {MAX_PASSAGES} passages")));
    }

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

    Ok(id)
}

/// Inserts an agent with its bases, inside `transaction`; refused as
/// [`agent`] says.
///
/// Called by: [`agent`], [`imported_agent`].
/// Calls: `update::bases_of`.
fn insert_agent(transaction: &Transaction<'_>, fields: &AgentFields) -> Result<i64, DbError> {
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
    super::update::bases_of(transaction, id, &fields.bases)?;

    Ok(id)
}

#[cfg(test)]
mod tests {
    //! Unit tests of `create.rs`: the import of an agent with its bases.

    use super::*;
    use crate::db::knowledge::read;
    use crate::model_old::knowledge::NewSource;
    use crate::test_support::standards::database;

    fn fields(name: &str) -> AgentFields {
        AgentFields {
            name: name.to_owned(),
            instructions: String::new(),
            glossary: String::new(),
            passages: 4,
            characters: 4000,
            bases: Vec::new(),
        }
    }

    fn new_base(name: &str) -> NewBase {
        let passage = Passage { page: Some(2), label: String::new(), text: "Roles are allocated.".to_owned() };
        let source = NewSource { kind: SourceKind::Pdf, name: "ISO 27002".to_owned(), passages: vec![passage] };

        NewBase { name: name.to_owned(), description: String::new(), sources: vec![source] }
    }

    #[test]
    fn an_imported_agent_gets_free_names_and_its_bases() {
        let test = database();
        base(&test.db, "ISO", "").unwrap();
        agent(&test.db, &fields("ISO agent")).unwrap();

        let id = imported_agent(&test.db, &fields("ISO agent"), &[new_base("ISO")]).unwrap();

        let found = read::agent(&test.db, id).unwrap().unwrap();
        assert_eq!(found.fields.name, "ISO agent (imported)");
        let summaries = read::bases(&test.db).unwrap();
        assert!(summaries.iter().any(|summary| summary.base.name == "ISO (imported)" && summary.passages == 1));
        assert_eq!(found.fields.bases.len(), 1);

        let again = imported_agent(&test.db, &fields("ISO agent"), &[new_base("ISO")]).unwrap();
        assert_eq!(read::agent(&test.db, again).unwrap().unwrap().fields.name, "ISO agent (imported 2)");
    }

    #[test]
    fn an_imported_name_keeps_within_its_limit() {
        let test = database();
        let long = "n".repeat(NAME_MAX);
        agent(&test.db, &fields(&long)).unwrap();

        let id = imported_agent(&test.db, &fields(&long), &[]).unwrap();

        let name = read::agent(&test.db, id).unwrap().unwrap().fields.name;
        assert!(name.ends_with(" (imported)") && name.chars().count() == NAME_MAX);
    }

    #[test]
    fn a_failed_import_stores_nothing() {
        let test = database();
        let mut too_long = new_base("Big");
        too_long.sources[0].passages =
            vec![Passage { page: None, label: String::new(), text: "x".to_owned() }; MAX_PASSAGES + 1];

        assert!(imported_agent(&test.db, &fields("A"), &[new_base("Small"), too_long]).is_err());

        assert!(read::bases(&test.db).unwrap().is_empty());
        assert!(read::agents(&test.db).unwrap().is_empty());
    }
}
