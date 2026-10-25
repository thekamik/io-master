//! The rows of the knowledge bases and the types of `model/knowledge.rs`
//! they become.

use crate::db::error::{Corrupt, DbError};
use crate::model::knowledge::{BaseSummary, Hit, KnowledgeBase, Passage, Source, SourceKind};

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

/// A passage found by a search, with the name of its source and its score.
///
/// Called by: `read::search`.
pub(super) fn hit(row: &rusqlite::Row<'_>) -> Result<Hit, DbError> {
    let page: Option<i64> = row.get("page")?;
    let page = match page {
        Some(number) => Some(u32::try_from(number).map_err(|error| DbError::Inconsistent(error.to_string()))?),
        None => None,
    };

    Ok(Hit {
        id: row.get("id")?,
        source: row.get("source")?,
        passage: Passage { page, label: row.get("label")?, text: row.get("text")? },
        score: row.get("score")?,
    })
}
