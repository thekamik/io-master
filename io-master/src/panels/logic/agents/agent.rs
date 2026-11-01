//! The agents, as their panel makes and changes them: an agent made, its
//! changes checked. The knowledge it gives with a question is a part of the
//! AI (`ai/models/ai/agent.rs`).
//!
//! An agent is a layer of knowledge over the model of an integration (user,
//! 2026-10-29): it names the integration that answers it, its instructions,
//! its knowledge bases (shared with other agents), its glossary and its
//! limits. It is changed freely: when its answers are not good enough, its
//! knowledge or its instructions are made better.

use crate::db::models::ai::agent::{MAX_CHARACTERS, MAX_TERMS, create_agent, read_agent, update_agent};
use crate::db::models::ai::integration::read_integration;
use crate::db::models::ai::rag::{MAX_BASES, MAX_FOUND, read_knowledge_base};
use crate::db::storage::user_database::UserDatabase;
use crate::model::ai::agent::{Agent, GlossaryEntry};
use crate::panels::logic::agents::error::AgentError;

/// The longest name of an agent, in characters.
const MAX_NAME: usize = 100;

/// The longest instructions of an agent, in characters.
pub const MAX_INSTRUCTIONS: usize = 20_000;

/// The longest term or meaning of a glossary, in characters.
const MAX_TERM: usize = 500;

/// The fewest characters of knowledge an agent gives: less than one
/// passage would give nothing whole.
const MIN_CHARACTERS: u32 = 1500;

/// The limits of a new agent: a few passages, a few pages of text.
const NEW_PASSAGES: u32 = 8;
const NEW_CHARACTERS: u32 = 12_000;

/// The instructions of a new agent, to be made its own by the user.
const NEW_INSTRUCTIONS: &str = "You are an evaluator of the security of a product. Base every statement on the \
knowledge given with the question; quote the text you rely on word for word and say where it comes from. When the \
knowledge is not enough, say what is missing.";

/// Makes a new agent of the integration `integration`, with the default
/// instructions and limits and no knowledge yet; gives its id.
///
/// Called by: `panels/views/agents/agent.rs` (a new agent).
/// Calls: `read_integration`, [`checked`], `create_agent`.
pub fn new_agent(db: &UserDatabase, name: &str, integration: i64) -> Result<i64, AgentError> {
    read_integration(db, integration)?;

    let agent = Agent {
        id: 0,
        name: name.to_owned(),
        integration,
        instructions: NEW_INSTRUCTIONS.to_owned(),
        knowledge_bases: Vec::new(),
        glossary: Vec::new(),
        max_passages: NEW_PASSAGES,
        max_characters: NEW_CHARACTERS,
    };

    Ok(create_agent(db, &checked(db, &agent)?)?)
}

/// Writes the agent `agent.id` anew: its name, integration, instructions,
/// knowledge bases, glossary and limits.
///
/// Called by: `panels/views/agents/agent.rs` (an agent saved).
/// Calls: `read_agent`, [`checked`], `update_agent`.
pub fn change_agent(db: &UserDatabase, agent: &Agent) -> Result<(), AgentError> {
    read_agent(db, agent.id)?;

    Ok(update_agent(db, &checked(db, agent)?)?)
}

/// The glossary with its text trimmed, the rows left empty taken out;
/// refused when a term has no meaning or a meaning no term, or a text is
/// over its limit.
///
/// Called by: [`checked`].
fn checked_glossary(glossary: &[GlossaryEntry]) -> Result<Vec<GlossaryEntry>, AgentError> {
    ensure!(glossary.len() <= MAX_TERMS, AgentError::Refused(format!("more than {MAX_TERMS} terms")));

    let mut kept = Vec::with_capacity(glossary.len());

    for entry in glossary {
        let (term, meaning) = (entry.term.trim(), entry.meaning.trim());

        if term.is_empty() && meaning.is_empty() {
            continue;
        }

        ensure!(
            !term.is_empty() && !meaning.is_empty(),
            AgentError::Refused(format!("the term \"{term}{meaning}\" needs both the term and its meaning"))
        );
        ensure!(
            term.chars().count() <= MAX_TERM && meaning.chars().count() <= MAX_TERM,
            AgentError::Refused(format!("a term or a meaning over {MAX_TERM} characters"))
        );
        kept.push(GlossaryEntry { term: term.to_owned(), meaning: meaning.to_owned() });
    }

    Ok(kept)
}

/// The agent with its text trimmed, checked: a name, an integration and
/// knowledge bases that exist, each base once, limits in their ranges.
///
/// Called by: [`new_agent`], [`change_agent`].
/// Calls: `read_integration`, `read_knowledge_base`, [`checked_glossary`].
fn checked(db: &UserDatabase, agent: &Agent) -> Result<Agent, AgentError> {
    let name = agent.name.trim();
    let passages = usize::try_from(agent.max_passages).unwrap_or(usize::MAX);

    ensure!(!name.is_empty(), AgentError::Refused("an agent without a name".to_owned()));
    ensure!(name.chars().count() <= MAX_NAME, AgentError::Refused(format!("a name over {MAX_NAME} characters")));
    ensure!(
        agent.instructions.chars().count() <= MAX_INSTRUCTIONS,
        AgentError::Refused(format!("instructions over {MAX_INSTRUCTIONS} characters"))
    );
    ensure!(
        (1..=MAX_FOUND).contains(&passages),
        AgentError::Refused(format!("the passages of a question: from 1 to {MAX_FOUND}"))
    );
    ensure!(
        (MIN_CHARACTERS..=MAX_CHARACTERS).contains(&agent.max_characters),
        AgentError::Refused(format!("the characters of a question: from {MIN_CHARACTERS} to {MAX_CHARACTERS}"))
    );
    ensure!(agent.knowledge_bases.len() <= MAX_BASES, AgentError::Refused("too many knowledge bases".to_owned()));
    read_integration(db, agent.integration)?;

    let mut bases = Vec::with_capacity(agent.knowledge_bases.len());

    for base in &agent.knowledge_bases {
        read_knowledge_base(db, *base)?;

        if !bases.contains(base) {
            bases.push(*base);
        }
    }

    Ok(Agent {
        id: agent.id,
        name: name.to_owned(),
        integration: agent.integration,
        instructions: agent.instructions.trim().to_owned(),
        knowledge_bases: bases,
        glossary: checked_glossary(&agent.glossary)?,
        max_passages: agent.max_passages,
        max_characters: agent.max_characters,
    })
}

#[cfg(test)]
mod tests {
    //! Tests of the agents: made, checked, changed.

    use super::*;

    use crate::db::models::ai::integration::create_integration;
    use crate::db::models::ai::rag::create_knowledge_base;
    use crate::model::ai::rag::KnowledgeBase;
    use crate::test_support::database::{database, integration};

    #[test]
    fn an_agent_is_made_checked_and_changed() {
        let (_directory, db) = database();
        let local = create_integration(&db, &integration()).unwrap();
        let id = new_agent(&db, "  agent_WU_1-1 ", local).unwrap();
        let base =
            create_knowledge_base(&db, &KnowledgeBase { id: 0, name: "CEM".to_owned(), description: String::new() })
                .unwrap();
        let mut agent = read_agent(&db, id).unwrap();

        assert_eq!((agent.name.as_str(), agent.instructions.as_str()), ("agent_WU_1-1", NEW_INSTRUCTIONS));

        agent.knowledge_bases = vec![base, base];
        agent.glossary = vec![
            GlossaryEntry { term: " TSF ".to_owned(), meaning: "TOE Security Functionality".to_owned() },
            GlossaryEntry { term: String::new(), meaning: " ".to_owned() },
        ];
        change_agent(&db, &agent).unwrap();

        let stored = read_agent(&db, id).unwrap();

        assert_eq!(stored.knowledge_bases, vec![base]);
        assert_eq!(
            stored.glossary,
            vec![GlossaryEntry { term: "TSF".to_owned(), meaning: "TOE Security Functionality".to_owned() }]
        );

        let refused = [
            Agent { name: " ".to_owned(), ..stored.clone() },
            Agent { max_passages: 0, ..stored.clone() },
            Agent { max_characters: 10, ..stored.clone() },
            Agent { glossary: vec![GlossaryEntry { term: "ST".to_owned(), meaning: String::new() }], ..stored.clone() },
        ];

        for agent in &refused {
            assert!(matches!(change_agent(&db, agent), Err(AgentError::Refused(_))));
        }

        assert!(matches!(
            change_agent(&db, &Agent { knowledge_bases: vec![base + 1], ..stored }),
            Err(AgentError::Db(_))
        ));
        assert!(new_agent(&db, "Another", local + 1).is_err());
    }
}
