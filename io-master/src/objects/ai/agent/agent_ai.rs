//! The agent (user, 2026-10-29): a layer of knowledge over the model of an
//! integration. The knowledge it gives with a question (RAG): the passages
//! of its knowledge bases that fit the question best, found with the help
//! of its glossary, within its limits; and the question it asks with it.
//!
//! ```text
//! a question ──► the words of the question, widened by the meanings of the terms of the glossary
//!            ──► read_passages in the bases of the agent, at most max_passages
//!            ──► the best first, as many as fit in max_characters (a passage never shortened)
//!
//! agent_question ──► the instructions of the agent + the task + the knowledge + the messages ──► Question
//! ```

use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::ai::agent::agent_model::GlossaryEntry;
use crate::objects::ai::agent::agent_store::read_agent;
use crate::objects::ai::chat::chat_model::Message;
use crate::objects::ai::integration::integration_ai::question::Question;
use crate::objects::ai::rag::rag_store::{MAX_FOUND, read_passages};
use crate::objects::ai_error::AiError;

/// The words of `text` in lower case: what lies between the signs that are
/// no letter or digit.
///
/// Input: `text`, a question or a term of a glossary, bounded by its caller.
/// Output: its words (`["the", "tsf", "protects"]` of "The TSF protects.").
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
/// Input: `question`, the words of the question ([`words`]); `term`, a term
/// of the glossary ("ST").
/// Output: `true` when the question names it; `false` for an empty term.
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
/// Input: `question`, the question as asked; `glossary`, the glossary of the
/// agent, checked when the agent was written (`agent_rules.rs`).
/// Output: the question with the meanings after it ("What is the TSF? TOE
/// Security Functionality").
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
/// Input: `agent`, the id of a stored agent; `question`, what the knowledge
/// is looked for with ("ADV_ARC.1-1 The developer shall ...").
/// Output: the passages given, the best first (`["The TSF shall ...", ...]`);
/// an error when the agent is missing or damaged or the database fails.
///
/// Called by: [`agent_question`]; `panels/views/agents/agent.rs`:
/// `agent_knowledge_test`.
/// Calls: `objects/ai/agent/agent_store.rs`: `read_agent`; [`widened`]; `objects/ai/rag/rag_store.rs`:
/// `read_passages`.
pub fn agent_knowledge(db: &UserDatabase, agent: i64, question: &str) -> Result<Vec<String>, AiError> {
    let agent = read_agent(db, agent)?;
    // The limits were checked when the agent was written
    // (`agent_rules.rs`); a negative one is a damaged agent, never a default.
    let count = usize::try_from(agent.max_passages)
        .map_err(|_| AiError::Refused(format!("the agent has {} passages", agent.max_passages)))?;
    let limit = usize::try_from(agent.max_characters)
        .map_err(|_| AiError::Refused(format!("the agent has {} characters", agent.max_characters)))?;

    ensure!(count <= MAX_FOUND, AiError::Refused(format!("the agent has {count} passages, over {MAX_FOUND}")));
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

/// The question the agent `agent` asks of its integration: its
/// instructions, then the task `task` (none when empty), then its knowledge
/// that fits `search`, then the messages `messages`, the last of them the
/// question of the user.
///
/// Input:
/// - `agent`: the id of a stored agent.
/// - `task`: what the agent is asked to do and in which form it answers
///   (the assessment of a work unit); empty for a chat.
/// - `search`: what its knowledge is looked for with ("ADV_ARC.1-1 The
///   developer shall ...").
/// - `work_unit`: the work unit the question is for; `None` outside an
///   evaluation.
/// - `messages`: the conversation, at least one message.
///
/// Output: the question, not prepared yet
/// (`objects/ai/integration/integration_ai/question.rs`: `prepare`). An error
/// when the agent cannot be read, or there is no message.
///
/// Called by: `objects/evaluation/work_unit/work_unit_ai/proposal.rs`: `question_of`;
/// `objects/evaluation/work_unit/work_unit_ai/assessment_question.rs`: `question_of`;
/// `objects/ai/chat/chat_ai.rs`: `chat_question`.
/// Calls: `objects/ai/agent/agent_store.rs`: `read_agent`; [`agent_knowledge`].
pub fn agent_question(
    db: &UserDatabase,
    agent: i64,
    task: &str,
    search: &str,
    work_unit: Option<i64>,
    messages: Vec<Message>,
) -> Result<Question, AiError> {
    ensure!(!messages.is_empty(), AiError::NoQuestion);

    let stored = read_agent(db, agent)?;
    let knowledge = agent_knowledge(db, agent, search)?;
    let mut system = stored.instructions;

    if !task.is_empty() {
        system.push_str("\n\n");
        system.push_str(task);
    }

    // The knowledge is named for what it was found for, so that the model
    // knows what it is about.
    if !knowledge.is_empty() {
        let found_for = if work_unit.is_some() { "this work unit" } else { "this question" };

        system.push_str(&format!("\n\nYour knowledge for {found_for}:\n\n"));
        system.push_str(&knowledge.join("\n\n---\n\n"));
    }

    Ok(Question { integration: stored.integration, agent: Some(agent), work_unit, system, messages })
}

#[cfg(test)]
mod tests {
    //! Tests of the knowledge of an agent: found with the glossary and
    //! within the limits.

    use super::*;

    use crate::objects::ai::agent::agent_model::Agent;
    use crate::objects::ai::agent::agent_store::{create_agent, update_agent};
    use crate::objects::ai::chat::chat_model::Speaker;
    use crate::objects::ai::integration::integration_store::create_integration;
    use crate::objects::ai::rag::rag_model::{KnowledgeBase, Source};
    use crate::objects::ai::rag::rag_store::{create_knowledge_base, create_source};
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

    #[test]
    fn the_question_of_an_agent_holds_its_instructions_its_task_and_its_knowledge() {
        let (_directory, db) = database();
        let local = create_integration(&db, &integration()).unwrap();
        let id = create_agent(&db, &agent(local)).unwrap();
        let base =
            create_knowledge_base(&db, &KnowledgeBase { id: 0, name: "CEM".to_owned(), description: String::new() })
                .unwrap();
        let text = "The developer shall describe the security architecture.".to_owned();

        create_source(&db, &Source { id: 0, knowledge_base: base, name: "notes".to_owned(), text }).unwrap();
        update_agent(&db, &Agent { id, knowledge_bases: vec![base], ..agent(local) }).unwrap();

        let message = Message { speaker: Speaker::User, text: "Assess it.".to_owned(), time: 0 };
        let question =
            agent_question(&db, id, "Answer in JSON.", "security architecture", Some(3), vec![message]).unwrap();

        assert!(question.system.starts_with("You assess work units of the CEM.\n\nAnswer in JSON."));
        assert!(question.system.contains("Your knowledge for this work unit:\n\nThe developer shall"));
        assert_eq!((question.integration, question.agent, question.work_unit), (local, Some(id), Some(3)));

        let chat = agent_question(&db, id, "", "routers", None, question.messages.clone()).unwrap();

        assert_eq!(chat.system, "You assess work units of the CEM.");
        assert!(matches!(agent_question(&db, id, "", "", None, Vec::new()), Err(AiError::NoQuestion)));
    }
}
