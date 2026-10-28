//! Reading knowledge bases, their sources, and searching their passages.

use rusqlite::{OptionalExtension, named_params};

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model_old::knowledge::{
    Agent, AgentPlace, AgentSummary, BaseSummary, Hit, KnowledgeBase, MAX_AGENT_PASSAGES, MAX_AGENTS, MAX_BASES,
    MAX_HITS, MAX_PASSAGES, MAX_QUOTES, MAX_SOURCES, Passage, Source, Trial,
};

use super::rows;

/// A limit as a parameter of SQL.
fn limit(max: usize) -> Result<i64, DbError> {
    i64::try_from(max).map_err(|error| DbError::Inconsistent(error.to_string()))
}

/// The bases by name, with the numbers of their sources and passages; at
/// most `MAX_BASES`.
///
/// Called by: `knowledge_list_get` and the commands that change the list (`views/knowledge/knowledge.rs`).
pub fn bases(db: &UserDb) -> Result<Vec<BaseSummary>, DbError> {
    let connection = db.connection();
    let mut statement = connection.prepare(
        "SELECT b.id AS id, b.name AS name, b.description AS description,
                (SELECT COUNT(*) FROM knowledge_sources s WHERE s.base_id = b.id) AS sources,
                (SELECT COALESCE(SUM(s.passages), 0) FROM knowledge_sources s WHERE s.base_id = b.id) AS passages
         FROM knowledge_bases b ORDER BY b.name COLLATE NOCASE LIMIT ?1",
    )?;
    let mut found = Vec::new();

    for row in statement.query_map([limit(MAX_BASES)?], |row| Ok(rows::summary(row)))? {
        found.push(row??);
    }

    Ok(found)
}

/// Base `id`; `None` when it does not exist.
///
/// Called by: the commands of a base (`views/knowledge/knowledge.rs`).
pub fn base(db: &UserDb, id: i64) -> Result<Option<KnowledgeBase>, DbError> {
    db.connection()
        .query_row("SELECT * FROM knowledge_bases WHERE id = ?1", [id], |row| Ok(rows::base(row)))
        .optional()?
        .transpose()
}

/// The sources of base `base`, in the order they were added; at most `MAX_SOURCES`.
///
/// Called by: `views/knowledge/knowledge_base/view.rs` (the screen of a base).
pub fn sources(db: &UserDb, base: i64) -> Result<Vec<Source>, DbError> {
    let connection = db.connection();
    let mut statement =
        connection.prepare("SELECT * FROM knowledge_sources WHERE base_id = ?1 ORDER BY id LIMIT ?2")?;
    let mut found = Vec::new();

    for row in statement.query_map(rusqlite::params![base, limit(MAX_SOURCES)?], |row| Ok(rows::source(row)))? {
        found.push(row??);
    }

    Ok(found)
}

/// The passages of source `source`, in the order they were stored; at most
/// `MAX_PASSAGES`.
///
/// Called by: `views/knowledge/agent_form/export.rs`.
pub fn passages_of(db: &UserDb, source: i64) -> Result<Vec<Passage>, DbError> {
    let connection = db.connection();
    let mut statement = connection
        .prepare("SELECT page, label, text FROM knowledge_passages WHERE source_id = ?1 ORDER BY rowid LIMIT ?2")?;
    let mut found = Vec::new();

    for row in statement.query_map(rusqlite::params![source, limit(MAX_PASSAGES)?], |row| Ok(rows::passage(row)))? {
        found.push(row??);
    }

    Ok(found)
}

/// The passages of base `base` that the FTS5 query `query`
/// (`model/knowledge.rs`, `search_query`) finds, the best first (BM25); at
/// most `max`, never more than `MAX_HITS`.
///
/// Called by: `views/knowledge/knowledge_search/search.rs`.
pub fn search(db: &UserDb, base: i64, query: &str, max: usize) -> Result<Vec<Hit>, DbError> {
    let connection = db.connection();
    let mut statement = connection.prepare(
        "SELECT knowledge_passages.rowid AS id, s.name AS source, knowledge_passages.page AS page,
                knowledge_passages.label AS label, knowledge_passages.text AS text,
                bm25(knowledge_passages) AS score
         FROM knowledge_passages JOIN knowledge_sources s ON s.id = knowledge_passages.source_id
         WHERE knowledge_passages MATCH :query AND s.base_id = :base
         ORDER BY score LIMIT :limit",
    )?;
    let parameters = named_params! { ":query": query, ":base": base, ":limit": limit(max.min(MAX_HITS))? };
    let mut found = Vec::new();

    for row in statement.query_map(parameters, |row| Ok(rows::hit(row, "id")))? {
        found.push(row??);
    }

    Ok(found)
}

/// The agents by name, with the numbers of their bases; at most `MAX_AGENTS`.
///
/// Called by: `agent_list_get` and the commands that change the list (`views/knowledge/knowledge.rs`).
pub fn agents(db: &UserDb) -> Result<Vec<AgentSummary>, DbError> {
    let connection = db.connection();
    let mut statement = connection.prepare(
        "SELECT a.id AS id, a.name AS name,
                (SELECT COUNT(*) FROM knowledge_agent_bases b WHERE b.agent_id = a.id) AS bases
         FROM knowledge_agents a ORDER BY a.name COLLATE NOCASE LIMIT ?1",
    )?;
    let mut found = Vec::new();

    for row in statement.query_map([limit(MAX_AGENTS)?], |row| Ok(rows::agent_summary(row)))? {
        found.push(row??);
    }

    Ok(found)
}

/// Agent `id` with the bases it draws on; `None` when it does not exist.
///
/// Called by: the commands of an agent (`views/knowledge/knowledge.rs`), the proving ground.
pub fn agent(db: &UserDb, id: i64) -> Result<Option<Agent>, DbError> {
    let connection = db.connection();
    let mut statement = connection
        .prepare("SELECT base_id FROM knowledge_agent_bases WHERE agent_id = ?1 ORDER BY base_id LIMIT ?2")?;
    let mut bases = Vec::new();

    for base in statement.query_map(rusqlite::params![id, limit(MAX_BASES)?], |row| row.get::<_, i64>(0))? {
        bases.push(base?);
    }

    let fields = connection
        .query_row("SELECT * FROM knowledge_agents WHERE id = ?1", [id], |row| Ok(rows::agent_fields(row, bases)))
        .optional()?
        .transpose()?;

    match fields {
        Some(fields) => Ok(Some(Agent { id, fields })),
        None => Ok(None),
    }
}

/// The passages of the bases of agent `agent` that the FTS5 query `query`
/// finds, the best first (BM25); at most `max`, never more than `MAX_HITS`.
///
/// Called by: `views/knowledge/proving_ground/` (the passages of a question).
pub fn search_agent(db: &UserDb, agent: i64, query: &str, max: usize) -> Result<Vec<Hit>, DbError> {
    let connection = db.connection();
    let mut statement = connection.prepare(
        "SELECT knowledge_passages.rowid AS id, s.name AS source, knowledge_passages.page AS page,
                knowledge_passages.label AS label, knowledge_passages.text AS text,
                bm25(knowledge_passages) AS score
         FROM knowledge_passages JOIN knowledge_sources s ON s.id = knowledge_passages.source_id
         WHERE knowledge_passages MATCH :query
           AND s.base_id IN (SELECT base_id FROM knowledge_agent_bases WHERE agent_id = :agent)
         ORDER BY score LIMIT :limit",
    )?;
    let parameters = named_params! { ":query": query, ":agent": agent, ":limit": limit(max.min(MAX_HITS))? };
    let mut found = Vec::new();

    for row in statement.query_map(parameters, |row| Ok(rows::hit(row, "id")))? {
        found.push(row??);
    }

    Ok(found)
}

/// The last trial of the proving ground of agent `agent`; `None` when it
/// has none.
///
/// Called by: `proving_ground_get` (`views/knowledge/knowledge.rs`).
pub fn trial(db: &UserDb, agent: i64) -> Result<Option<Trial>, DbError> {
    let connection = db.connection();
    let asked = connection
        .query_row("SELECT question, answer FROM knowledge_trials WHERE agent_id = ?1", [agent], |row| {
            Ok((row.get::<_, String>("question")?, row.get::<_, String>("answer")?))
        })
        .optional()?;
    let Some((question, answer)) = asked else {
        return Ok(None);
    };

    let mut statement =
        connection.prepare("SELECT * FROM knowledge_trial_passages WHERE agent_id = ?1 ORDER BY position LIMIT ?2")?;
    let mut passages = Vec::new();

    for row in statement
        .query_map(rusqlite::params![agent, limit(MAX_AGENT_PASSAGES)?], |row| Ok(rows::hit(row, "passage_id")))?
    {
        passages.push(row??);
    }

    let mut statement =
        connection.prepare("SELECT * FROM knowledge_trial_quotes WHERE agent_id = ?1 ORDER BY position LIMIT ?2")?;
    let mut quotes = Vec::new();

    for row in statement.query_map(rusqlite::params![agent, limit(MAX_QUOTES)?], |row| Ok(rows::trial_quote(row)))? {
        quotes.push(row??);
    }

    Ok(Some(Trial { question, passages, answer, quotes }))
}

/// The agent attached to object `owner` of `place`; `None` when it has none.
///
/// Called by: the views that show or use the agent of a project, a catalog or a chat.
pub fn attachment(db: &UserDb, place: AgentPlace, owner: &str) -> Result<Option<i64>, DbError> {
    Ok(db
        .connection()
        .query_row(
            "SELECT agent_id FROM knowledge_attachments WHERE place = :place AND owner = :owner",
            named_params! { ":place": place.name(), ":owner": owner },
            |row| row.get(0),
        )
        .optional()?)
}

#[cfg(test)]
mod tests {
    //! Tests of the knowledge bases in the database: stored, read, searched,
    //! changed and deleted (`create.rs`, `update.rs`, `delete.rs` with them).

    use super::*;
    use crate::db::knowledge::{create, delete, update};
    use crate::model_old::knowledge::{Passage, SourceKind};
    use crate::test_support::standards::database;

    fn passage(page: u32, text: &str) -> Passage {
        Passage { page: Some(page), label: String::new(), text: text.to_owned() }
    }

    #[test]
    fn a_base_keeps_its_sources_and_finds_its_passages() {
        let test = database();
        let base = create::base(&test.db, "Common Criteria", "CC and CEM").unwrap();
        let other = create::base(&test.db, "ETSI", "").unwrap();
        let passages = [
            passage(12, "The TSF shall generate cryptographic keys (FCS_CKM.1)."),
            passage(13, "Audit data generation records the events."),
        ];
        create::source(&test.db, base, SourceKind::Pdf, "CC Part 2", &passages).unwrap();
        create::source(&test.db, other, SourceKind::Note, "Other", &[passage(1, "cryptographic keys")]).unwrap();

        let hits = search(&test.db, base, "\"FCS CKM 1\" OR \"keys\"", 10).unwrap();
        assert_eq!(hits.len(), 1, "only the passages of the base");
        assert_eq!((hits[0].source.as_str(), hits[0].passage.page), ("CC Part 2", Some(12)));

        let summaries = bases(&test.db).unwrap();
        assert_eq!(summaries[0].base.name, "Common Criteria");
        assert_eq!((summaries[0].sources, summaries[0].passages), (1, 2));
        assert_eq!(sources(&test.db, base).unwrap()[0].kind, SourceKind::Pdf);
    }

    #[test]
    fn a_name_is_unique() {
        let test = database();
        let base = create::base(&test.db, "CC", "").unwrap();
        let other = create::base(&test.db, "ETSI", "").unwrap();

        assert!(create::base(&test.db, "CC", "").is_err());
        assert!(update::base(&test.db, other, "CC", "").is_err());
        assert!(update::base(&test.db, base, "CC 2022", "Part 1 to 5").unwrap());
        assert_eq!(base_of(&test, base).description, "Part 1 to 5");
        assert!(!update::base(&test.db, 99, "X", "").unwrap());
    }

    fn base_of(test: &crate::test_support::standards::TestDb, id: i64) -> KnowledgeBase {
        super::base(&test.db, id).unwrap().unwrap()
    }

    #[test]
    fn a_deleted_source_or_base_takes_its_passages() {
        let test = database();
        let base = create::base(&test.db, "CC", "").unwrap();
        let first = create::source(&test.db, base, SourceKind::Note, "A", &[passage(1, "alpha words")]).unwrap();
        create::source(&test.db, base, SourceKind::Note, "B", &[passage(1, "beta words")]).unwrap();

        assert!(delete::source(&test.db, base, first).unwrap());
        assert!(!delete::source(&test.db, base, first).unwrap());
        assert!(search(&test.db, base, "\"alpha\"", 10).unwrap().is_empty());
        assert_eq!(search(&test.db, base, "\"words\"", 10).unwrap().len(), 1);

        assert!(delete::base(&test.db, base).unwrap());
        assert!(super::base(&test.db, base).unwrap().is_none());
        let left: i64 =
            test.db.connection().query_row("SELECT COUNT(*) FROM knowledge_passages", [], |row| row.get(0)).unwrap();
        assert_eq!(left, 0);
    }

    #[test]
    fn an_agent_is_attached_and_goes_with_its_agent() {
        let test = database();
        let fields = crate::model_old::knowledge::AgentFields {
            name: "CC".to_owned(),
            instructions: String::new(),
            glossary: String::new(),
            passages: 4,
            characters: 4000,
            bases: Vec::new(),
        };
        let agent = create::agent(&test.db, &fields).unwrap();

        update::attachment(&test.db, AgentPlace::Chat, "7", Some(agent)).unwrap();
        assert_eq!(attachment(&test.db, AgentPlace::Chat, "7").unwrap(), Some(agent));
        assert_eq!(attachment(&test.db, AgentPlace::Project, "7").unwrap(), None);
        assert!(update::attachment(&test.db, AgentPlace::Chat, "8", Some(99)).is_err());

        delete::agent(&test.db, agent).unwrap();
        assert_eq!(attachment(&test.db, AgentPlace::Chat, "7").unwrap(), None);
    }

    #[test]
    fn a_source_needs_its_base() {
        let test = database();

        assert!(create::source(&test.db, 7, SourceKind::Note, "A", &[]).is_err());
    }
}
