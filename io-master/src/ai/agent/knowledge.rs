//! The knowledge an agent gives with a question (RAG, user 2026-10-29):
//! the passages of its knowledge bases that fit the question best, found
//! with the help of its glossary, within its limits.
//!
//! ```text
//! a question ──► the words of the question, widened by the meanings of the terms of the glossary
//!            ──► read_passages in the bases of the agent, at most max_passages
//!            ──► the best first, as many as fit in max_characters (a passage never shortened)
//! ```

use crate::ai::error::AiError;
use crate::db::models::ai::agent::read_agent;
use crate::db::models::ai::rag::{MAX_FOUND, read_passages};
use crate::db::storage::user_database::UserDatabase;
use crate::model::ai::agent::GlossaryEntry;

/// The words of `text` in lower case: what lies between the signs that are
/// no letter or digit.
///
/// Called by: [`names_term`], [`widened`].
fn words(text: &str) -> Vec<String> {
    let mut found = Vec::new();

    // The closure tells `split` where a word ends (a library place, rule 9).
    for word in text.split(|character: char| !character.is_alphanumeric()) {
        if !word.is_empty() {
            found.push(word.to_lowercase());
        }
    }

    found
}

/// Whether the question names the term: the words of the term follow one
/// another among the words of the question, whole (user, 2026-10-29: "ST"
/// is not named by "test").
///
/// Called by: [`widened`].
/// Calls: [`words`].
fn names_term(question: &[String], term: &str) -> bool {
    let term = words(term);

    if term.is_empty() {
        return false;
    }

    for window in question.windows(term.len()) {
        if window == term.as_slice() {
            return true;
        }
    }

    false
}

/// The question widened by the glossary of the agent: the meaning of every
/// term the question names is added, so that the search finds a text that
/// writes the term out ("TSF" also finds "TOE Security Functionality").
///
/// Called by: [`agent_knowledge`].
/// Calls: [`words`], [`names_term`].
fn widened(question: &str, glossary: &[GlossaryEntry]) -> String {
    let asked = words(question);
    let mut widened = question.to_owned();

    for entry in glossary {
        if names_term(&asked, &entry.term) {
            widened.push(' ');
            widened.push_str(&entry.meaning);
        }
    }

    widened
}

/// The knowledge the agent `agent` gives with the question `question`: the
/// passages of its bases that fit it best, at most its `max_passages`, as
/// many of them, the best first, as fit in its `max_characters`. A passage
/// is never shortened: one that does not fit is left out.
///
/// Called by: `panels/views/agents/agent.rs`: `agent_knowledge_test`;
/// `panels/logic/evaluation_plan/fill.rs`: `question_of`;
/// `panels/logic/evaluation/question.rs`: `assessment_question`;
/// `panels/logic/chat/chats.rs`: `chat_question`.
/// Calls: `db/models/ai/agent.rs`: `read_agent`; [`widened`];
/// `db/models/ai/rag.rs`: `read_passages`.
pub fn agent_knowledge(db: &UserDatabase, agent: i64, question: &str) -> Result<Vec<String>, AiError> {
    let agent = read_agent(db, agent)?;
    let count = usize::try_from(agent.max_passages).unwrap_or(MAX_FOUND);
    let limit = usize::try_from(agent.max_characters).unwrap_or(0);
    let found = read_passages(db, &agent.knowledge_bases, &widened(question, &agent.glossary), count)?;
    let mut given = Vec::with_capacity(found.len());
    let mut characters = 0;

    for passage in found {
        let length = passage.chars().count();

        if characters + length <= limit {
            characters += length;
            given.push(passage);
        }
    }

    Ok(given)
}

#[cfg(test)]
mod tests {
    //! Tests of the knowledge of an agent: found with the glossary and
    //! within the limits.

    use super::*;

    use crate::db::models::ai::agent::{create_agent, update_agent};
    use crate::db::models::ai::integration::create_integration;
    use crate::db::models::ai::rag::{create_knowledge_base, create_source};
    use crate::model::ai::rag::{KnowledgeBase, Source};
    use crate::test_support::database::{agent, database, integration};

    #[test]
    fn the_knowledge_of_an_agent_is_found_with_its_glossary_and_fits_its_limit() {
        let (_directory, db) = database();
        let local = create_integration(&db, &integration()).unwrap();
        let id = create_agent(&db, &agent(local)).unwrap();
        let base =
            create_knowledge_base(&db, &KnowledgeBase { id: 0, name: "CEM".to_owned(), description: String::new() })
                .unwrap();
        let paragraph = "The TOE Security Functionality enforces the policy.";
        let text = format!("{paragraph}\n\n{}\n\nUnrelated words about routers.", "x".repeat(1400));

        create_source(&db, &Source { id: 0, knowledge_base: base, name: "notes".to_owned(), text }).unwrap();

        let mut agent = read_agent(&db, id).unwrap();

        agent.knowledge_bases = vec![base];
        update_agent(&db, &agent).unwrap();

        // Without the glossary "TSF" finds nothing; with it, the passage
        // that writes it out.
        assert!(agent_knowledge(&db, id, "TSF").unwrap().is_empty());

        agent.glossary = vec![GlossaryEntry { term: "TSF".to_owned(), meaning: "Security Functionality".to_owned() }];
        update_agent(&db, &agent).unwrap();

        let found = agent_knowledge(&db, id, "TSF").unwrap();

        assert_eq!(found.len(), 1);
        assert!(found[0].contains(paragraph));
        assert!(found.iter().map(String::len).sum::<usize>() <= 12_000);
    }

    #[test]
    fn a_term_is_named_only_by_whole_words() {
        let glossary = [
            GlossaryEntry { term: "ST".to_owned(), meaning: "Security Target".to_owned() },
            GlossaryEntry { term: "TOE scope".to_owned(), meaning: "the boundary".to_owned() },
        ];

        assert_eq!(widened("the test of the TOE", &glossary), "the test of the TOE");
        assert_eq!(widened("Read the ST.", &glossary), "Read the ST. Security Target");
        assert_eq!(widened("the toe Scope", &glossary), "the toe Scope the boundary");
    }
}
