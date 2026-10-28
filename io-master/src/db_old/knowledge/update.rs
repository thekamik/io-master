//! Changing a knowledge base, an agent, the last trial of an agent.

use rusqlite::{Transaction, named_params};

use crate::db_old::error::DbError;
use crate::db_old::user::database::UserDb;
use crate::model_old::knowledge::{AgentFields, AgentPlace, MAX_BASES, Trial};

/// A limit as a number of SQL.
fn limit(max: usize) -> Result<i64, DbError> {
    i64::try_from(max).map_err(|error| DbError::Inconsistent(error.to_string()))
}

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

/// Gives agent `id` the fields `fields` and the bases they name; gives
/// whether it exists. Refused when another agent has that name or a base
/// does not exist. One transaction.
///
/// Called by: `agent_form_save` (`views/knowledge/knowledge.rs`).
/// Calls: [`bases_of`].
pub fn agent(db: &UserDb, id: i64, fields: &AgentFields) -> Result<bool, DbError> {
    let transaction = db.connection().unchecked_transaction()?;
    let taken: bool = transaction.query_row(
        "SELECT COUNT(*) > 0 FROM knowledge_agents WHERE name = :name AND id != :id",
        named_params! { ":name": fields.name, ":id": id },
        |row| row.get(0),
    )?;

    if taken {
        return Err(DbError::Inconsistent("an agent of this name exists already".to_owned()));
    }

    let changed = transaction.execute(
        "UPDATE knowledge_agents SET name = :name, instructions = :instructions, glossary = :glossary,
                                     passages = :passages, characters = :characters
         WHERE id = :id",
        named_params! {
            ":name": fields.name,
            ":instructions": fields.instructions,
            ":glossary": fields.glossary,
            ":passages": limit(fields.passages)?,
            ":characters": limit(fields.characters)?,
            ":id": id,
        },
    )?;

    if changed > 0 {
        bases_of(&transaction, id, &fields.bases)?;
    }

    transaction.commit()?;

    Ok(changed > 0)
}

/// Makes `bases` the bases of agent `agent`, inside `transaction`; refused
/// when there are more than `MAX_BASES` or a base does not exist.
///
/// Called by: [`agent`], `create::agent`.
pub(super) fn bases_of(transaction: &Transaction<'_>, agent: i64, bases: &[i64]) -> Result<(), DbError> {
    if bases.len() > MAX_BASES {
        return Err(DbError::Inconsistent(format!("an agent draws on at most {MAX_BASES} bases")));
    }

    transaction.execute("DELETE FROM knowledge_agent_bases WHERE agent_id = ?1", [agent])?;

    for base in bases {
        let exists: bool =
            transaction
                .query_row("SELECT COUNT(*) > 0 FROM knowledge_bases WHERE id = ?1", [base], |row| row.get(0))?;

        if !exists {
            return Err(DbError::NoRow { table: "knowledge_bases", id: *base });
        }

        transaction.execute(
            "INSERT OR IGNORE INTO knowledge_agent_bases (agent_id, base_id) VALUES (:agent, :base)",
            named_params! { ":agent": agent, ":base": base },
        )?;
    }

    Ok(())
}

/// Keeps `trial` as the last trial of agent `agent`, in the place of the
/// one before. One transaction.
///
/// Called by: `views/knowledge/proving_ground/ask.rs`.
pub fn trial(db: &UserDb, agent: i64, trial: &Trial) -> Result<(), DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    super::delete::trial_of(&transaction, agent)?;
    transaction.execute(
        "INSERT INTO knowledge_trials (agent_id, question, answer, asked)
         VALUES (:agent, :question, :answer, :asked)",
        named_params! { ":agent": agent, ":question": trial.question, ":answer": trial.answer, ":asked": crate::model_old::time::now() },
    )?;

    for (position, hit) in trial.passages.iter().enumerate() {
        transaction.execute(
            "INSERT INTO knowledge_trial_passages (agent_id, position, passage_id, source, page, label, text, score)
             VALUES (:agent, :position, :passage, :source, :page, :label, :text, :score)",
            named_params! {
                ":agent": agent, ":position": limit(position)?, ":passage": hit.id, ":source": hit.source,
                ":page": hit.passage.page, ":label": hit.passage.label, ":text": hit.passage.text, ":score": hit.score,
            },
        )?;
    }

    for (position, quote) in trial.quotes.iter().enumerate() {
        transaction.execute(
            "INSERT INTO knowledge_trial_quotes (agent_id, position, passage, quote, found)
             VALUES (:agent, :position, :passage, :quote, :found)",
            named_params! {
                ":agent": agent, ":position": limit(position)?, ":passage": limit(quote.passage)?,
                ":quote": quote.quote, ":found": quote.found,
            },
        )?;
    }

    transaction.commit()?;

    Ok(())
}

/// Attaches agent `agent` to object `owner` of `place` (none: `None`, the
/// attachment removed); refused when the agent does not exist. One
/// transaction.
///
/// Called by: the commands that choose the agent of a project, a catalog or a chat.
pub fn attachment(db: &UserDb, place: AgentPlace, owner: &str, agent: Option<i64>) -> Result<(), DbError> {
    let transaction = db.connection().unchecked_transaction()?;
    let keys = named_params! { ":place": place.name(), ":owner": owner };

    transaction.execute("DELETE FROM knowledge_attachments WHERE place = :place AND owner = :owner", keys)?;

    if let Some(id) = agent {
        let exists: bool =
            transaction.query_row("SELECT COUNT(*) > 0 FROM knowledge_agents WHERE id = ?1", [id], |row| row.get(0))?;

        if !exists {
            return Err(DbError::NoRow { table: "knowledge_agents", id });
        }

        transaction.execute(
            "INSERT INTO knowledge_attachments (place, owner, agent_id) VALUES (:place, :owner, :agent)",
            named_params! { ":place": place.name(), ":owner": owner, ":agent": id },
        )?;
    }

    transaction.commit()?;

    Ok(())
}
