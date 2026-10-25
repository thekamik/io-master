//! The passages an agent with knowledge gives the model with a question
//! (step 57.3 of docs/todo.md): the question widened by the glossary of the
//! agent, made a query of FTS5, searched in the bases of the agent, and the
//! best passages chosen within its limits. A component of Rust only: the
//! proving ground of the view Knowledge (RAG) uses it now, the works of the
//! AI of the other views when an agent is attached to them (57.4, 57.6).

use crate::app::error::AppError;
use crate::db::knowledge as db_knowledge;
use crate::db::user::database::UserDb;
use crate::model::knowledge::{Agent, Hit, MAX_HITS, QUESTION_MAX, chosen, glossary, search_query, widened};

/// Agent `id`; refused when it does not exist.
///
/// Called by: the commands of an agent (`views/knowledge/knowledge.rs`), the proving ground.
pub fn agent(db: &UserDb, id: i64) -> Result<Agent, AppError> {
    match db_knowledge::read::agent(db, id)? {
        Some(agent) => Ok(agent),
        None => Err(AppError::Invalid("the agent does not exist".to_owned())),
    }
}

/// The passages `agent` gives the model with `question`, the best first:
/// none when the question has no word to look for or the agent no base.
///
/// Called by: `views/knowledge/proving_ground/ask.rs`, `proving_ground_find` (`views/knowledge/knowledge.rs`).
/// Calls: `glossary`, `widened`, `search_query`, `db_knowledge::read::search_agent`, `chosen`.
pub fn passages(db: &UserDb, agent: &Agent, question: &str) -> Result<Vec<Hit>, AppError> {
    ensure!(!question.trim().is_empty(), AppError::Invalid("the question is empty".to_owned()));
    ensure!(
        question.chars().count() <= QUESTION_MAX,
        AppError::Invalid(format!("a question is at most {QUESTION_MAX} characters"))
    );

    let entries = glossary(&agent.fields.glossary).map_err(|error| AppError::Invalid(error.to_string()))?;
    let Some(query) = search_query(&widened(question, &entries)) else {
        return Ok(Vec::new());
    };
    let hits = db_knowledge::read::search_agent(db, agent.id, &query, MAX_HITS)?;

    Ok(chosen(hits, agent.fields.passages, agent.fields.characters))
}

#[cfg(test)]
mod tests {
    //! Unit tests of `find.rs`.

    use super::*;

    use crate::model::knowledge::{AgentFields, Passage, SourceKind};
    use crate::test_support::standards::database;

    #[test]
    fn the_glossary_finds_what_the_question_names_otherwise() {
        let test = database();
        let base = db_knowledge::create::base(&test.db, "CC", "").unwrap();
        let other = db_knowledge::create::base(&test.db, "Other", "").unwrap();
        let passage = |text: &str| Passage { page: Some(3), label: String::new(), text: text.to_owned() };
        db_knowledge::create::source(
            &test.db,
            base,
            SourceKind::Pdf,
            "CC Part 1",
            &[passage("The TOE Security Functionality enforces the SFRs.")],
        )
        .unwrap();
        db_knowledge::create::source(&test.db, other, SourceKind::Note, "Other", &[passage("TSF of another base.")])
            .unwrap();
        let fields = AgentFields {
            name: "CC".to_owned(),
            instructions: String::new(),
            glossary: "TSF = TOE Security Functionality".to_owned(),
            passages: 5,
            characters: 5000,
            bases: vec![base],
        };
        let id = db_knowledge::create::agent(&test.db, &fields).unwrap();
        let found = agent(&test.db, id).unwrap();

        let hits = passages(&test.db, &found, "What does the TSF enforce?").unwrap();

        assert_eq!(hits.len(), 1, "only the bases of the agent");
        assert_eq!(hits[0].source, "CC Part 1");
        assert!(passages(&test.db, &found, " ").is_err());
        assert!(agent(&test.db, 99).is_err());
    }
}
