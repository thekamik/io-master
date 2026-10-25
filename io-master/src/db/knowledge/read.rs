//! Reading knowledge bases, their sources, and searching their passages.

use rusqlite::{OptionalExtension, named_params};

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model::knowledge::{BaseSummary, Hit, KnowledgeBase, MAX_BASES, MAX_HITS, MAX_SOURCES, Source};

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

    for row in statement.query_map(parameters, |row| Ok(rows::hit(row)))? {
        found.push(row??);
    }

    Ok(found)
}

#[cfg(test)]
mod tests {
    //! Tests of the knowledge bases in the database: stored, read, searched,
    //! changed and deleted (`create.rs`, `update.rs`, `delete.rs` with them).

    use super::*;
    use crate::db::knowledge::{create, delete, update};
    use crate::model::knowledge::{Passage, SourceKind};
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
    fn a_source_needs_its_base() {
        let test = database();

        assert!(create::source(&test.db, 7, SourceKind::Note, "A", &[]).is_err());
    }
}
