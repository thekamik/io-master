//! The rows of the knowledge bases and the types of `model/knowledge.rs`
//! they become.

use crate::db::error::{Corrupt, DbError};
use crate::model_old::knowledge::{
    AgentFields, AgentSummary, BaseSummary, Hit, KnowledgeBase, Passage, Source, SourceKind, TrialQuote,
};

/// A count of a column `column` as a number of the program.
fn count(row: &rusqlite::Row<'_>, column: &str) -> Result<usize, DbError> {
    let value: i64 = row.get(column)?;

    usize::try_from(value).map_err(|error| DbError::Inconsistent(error.to_string()))
}

/// A row of `knowledge_bases`.
///
/// Called by: `read::base`, [`summary`].
pub(super) fn base(row: &rusqlite::Row<'_>) -> Result<KnowledgeBase, DbError> {
    Ok(KnowledgeBase { id: row.get("id")?, name: row.get("name")?, description: row.get("description")? })
}

/// A row of `knowledge_bases` with the numbers of its sources and passages.
///
/// Called by: `read::bases`.
pub(super) fn summary(row: &rusqlite::Row<'_>) -> Result<BaseSummary, DbError> {
    Ok(BaseSummary { base: base(row)?, sources: count(row, "sources")?, passages: count(row, "passages")? })
}

/// A row of `knowledge_sources`; a kind the program never writes is corrupt.
///
/// Called by: `read::sources`.
pub(super) fn source(row: &rusqlite::Row<'_>) -> Result<Source, DbError> {
    let kind: String = row.get("kind")?;
    let Some(known) = SourceKind::from_name(&kind) else {
        return Err(DbError::Corrupt(Corrupt { table: "knowledge_sources", column: "kind", value: kind }));
    };

    Ok(Source {
        id: row.get("id")?,
        kind: known,
        name: row.get("name")?,
        passages: count(row, "passages")?,
        added: row.get("added")?,
    })
}

/// A stored passage: its page, label and text.
///
/// Called by: `read::passages_of`, [`hit`].
pub(super) fn passage(row: &rusqlite::Row<'_>) -> Result<Passage, DbError> {
    let page: Option<i64> = row.get("page")?;
    let page = match page {
        Some(number) => Some(u32::try_from(number).map_err(|error| DbError::Inconsistent(error.to_string()))?),
        None => None,
    };

    Ok(Passage { page, label: row.get("label")?, text: row.get("text")? })
}

/// A passage found by a search (or given to a trial), with the name of its
/// source and its score; its id in the column `column`.
///
/// Called by: `read::search`, `read::search_agent`, `read::trial`.
pub(super) fn hit(row: &rusqlite::Row<'_>, column: &str) -> Result<Hit, DbError> {
    Ok(Hit { id: row.get(column)?, source: row.get("source")?, passage: passage(row)?, score: row.get("score")? })
}

/// A row of `knowledge_agents` with the bases `bases` it draws on.
///
/// Called by: `read::agent`.
pub(super) fn agent_fields(row: &rusqlite::Row<'_>, bases: Vec<i64>) -> Result<AgentFields, DbError> {
    Ok(AgentFields {
        name: row.get("name")?,
        instructions: row.get("instructions")?,
        glossary: row.get("glossary")?,
        passages: count(row, "passages")?,
        characters: count(row, "characters")?,
        bases,
    })
}

/// A row of `knowledge_agents` with the number of its bases.
///
/// Called by: `read::agents`.
pub(super) fn agent_summary(row: &rusqlite::Row<'_>) -> Result<AgentSummary, DbError> {
    Ok(AgentSummary { id: row.get("id")?, name: row.get("name")?, bases: count(row, "bases")? })
}

/// A row of `knowledge_trial_quotes`.
///
/// Called by: `read::trial`.
pub(super) fn trial_quote(row: &rusqlite::Row<'_>) -> Result<TrialQuote, DbError> {
    Ok(TrialQuote { passage: count(row, "passage")?, quote: row.get("quote")?, found: row.get("found")? })
}
