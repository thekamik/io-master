//! The search of a knowledge base (step 57.2 of docs/todo.md): the words of
//! the evaluator made a query of FTS5 (`model/knowledge.rs`,
//! `search_query`) and the passages of the base ranked by BM25. Step 57.3
//! uses the same search to give an agent its passages.

use crate::app::error::AppError;
use crate::db::knowledge as db_knowledge;
use crate::db::user::database::UserDb;
use crate::model_old::knowledge::{Hit, MAX_HITS, QUERY_MAX, search_query};

/// The passages of base `base` that fit the text `text`, the best first,
/// at most `MAX_HITS`; none when the text has no word to look for.
///
/// Called by: `knowledge_search_run` (`knowledge.rs`).
/// Calls: `search_query`, `db_knowledge::read::search`.
pub fn search(db: &UserDb, base: i64, text: &str) -> Result<Vec<Hit>, AppError> {
    ensure!(
        text.chars().count() <= QUERY_MAX,
        AppError::Invalid(format!("a search is at most {QUERY_MAX} characters"))
    );
    ensure!(
        db_knowledge::read::base(db, base)?.is_some(),
        AppError::Invalid("the knowledge base does not exist".to_owned())
    );

    let Some(query) = search_query(text) else {
        return Ok(Vec::new());
    };

    Ok(db_knowledge::read::search(db, base, &query, MAX_HITS)?)
}

#[cfg(test)]
mod tests {
    //! Unit tests of `search.rs`.

    use super::*;

    use crate::model_old::knowledge::{Passage, SourceKind};
    use crate::test_support::standards::database;

    #[test]
    fn the_passage_with_the_rarer_words_comes_first() {
        let test = database();
        let base = db_knowledge::create::base(&test.db, "CC", "").unwrap();
        let passages = [
            Passage { page: Some(1), label: String::new(), text: "The TOE and the TSF and the TOE.".to_owned() },
            Passage { page: Some(2), label: String::new(), text: "FCS_CKM.1 key generation of the TSF.".to_owned() },
        ];
        db_knowledge::create::source(&test.db, base, SourceKind::Pdf, "CC Part 2", &passages).unwrap();

        let hits = search(&test.db, base, "How are keys generated? FCS_CKM.1 of the TSF").unwrap();

        assert_eq!(hits[0].passage.page, Some(2));
        assert!(search(&test.db, base, "- ; ,").unwrap().is_empty());
        assert!(search(&test.db, 99, "keys").is_err());
        assert!(search(&test.db, base, &"w".repeat(QUERY_MAX + 1)).is_err());
    }
}
