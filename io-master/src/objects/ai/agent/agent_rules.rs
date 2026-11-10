//! The rules of an agent (`Agent`): an agent made, its changes checked. The
//! knowledge it gives with a question is a part of the AI
//! (`objects/ai/agent/agent_ai.rs`).
//!
//! An agent is a layer of knowledge over the model of an integration (user,
//! 2026-10-29): it names the integration that answers it, its instructions,
//! its knowledge bases (shared with other agents), its glossary, its limits
//! and its role with the parameters of its work (user, 2026-11-02: everything
//! that shapes a question and its result is in the agent, so that a
//! laboratory tunes it without a change of the program). It is changed freely
//! but for its role: when its answers are not good enough, its knowledge, its
//! instructions or the tasks of its role are made better.

use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::ai::agent::agent_model::{Agent, GlossaryEntry};
use crate::objects::ai::agent::agent_store::{
    MAX_ANSWER_TOKENS, MAX_CHARACTERS, MAX_TEMPLATE, MAX_TERMS, create_agent, read_agent, update_agent,
};
use crate::objects::ai::agent_roles::agent_roles_rules::{checked_role, new_role, role_warnings, template_warnings};
use crate::objects::ai::agent_roles::agent_roles_store::role_name;
use crate::objects::ai::integration::integration_store::read_integration;
use crate::objects::ai::rag::rag_store::{MAX_BASES, MAX_FOUND, read_knowledge_base};
use crate::objects::error::DomainError;
use crate::objects::text::checked_text;

/// The longest name of an agent, in characters.
const MAX_NAME: usize = 100;

/// The longest instructions of an agent, in characters.
pub const MAX_INSTRUCTIONS: usize = 20_000;

/// The longest term or meaning of a glossary, in characters.
const MAX_TERM: usize = 500;

/// The fewest characters of knowledge an agent gives: less than one
/// passage would give nothing whole.
pub const MIN_CHARACTERS: u32 = 1500;

/// The limits of a new agent: a few passages, a few pages of text; the
/// characters hold each passage of the length a new base cuts
/// (`objects/ai/rag/rag_store.rs`: `PASSAGE_CHARS`) with the one that follows it.
const NEW_PASSAGES: u32 = 8;
const NEW_CHARACTERS: u32 = 24_000;

/// The longest answer a new agent asks for, in tokens, and the shortest one
/// an agent may ask for: less leaves no room for an assessment.
const NEW_ANSWER_TOKENS: u32 = 16_000;
pub const MIN_ANSWER_TOKENS: u32 = 256;

/// The instructions of a new agent, to be made its own by the user.
const NEW_INSTRUCTIONS: &str = "You are an experienced evaluator of the security of products. Use the knowledge \
given with a question, when there is any, to read the requirement as the standard and the laboratory mean it; judge \
the product only by its own documents and proofs. Be precise and brief.";

/// How a new agent lays out its message to the model: its instructions, its
/// knowledge, its task last, so that the form of the answer is the last thing
/// the model reads before the question; a part left empty takes no room
/// (`objects/ai/agent/agent_ai.rs`: `agent_question`).
pub const NEW_SYSTEM_TEMPLATE: &str = "{instructions}\n\n{knowledge}\n\n{task}";

/// How a new agent introduces its knowledge, and each passage of it under the
/// name of its source, so that the model can say what it relied on.
pub const NEW_KNOWLEDGE_TEMPLATE: &str = "Your knowledge for this question:\n\n{passages}";
pub const NEW_PASSAGE_TEMPLATE: &str = "=== {source} ===\n{text}";

/// What a new agent searches its knowledge with in a chat: the question of
/// the user before and the question, so that a short follow-up ("and in the
/// ST?") still finds what the talk is about.
pub const NEW_CHAT_SEARCH: &str = "{previous} {question}";

/// The placeholders without which the message of an agent loses its sense: a
/// template without one of them is saved with a warning (user, 2026-11-02).
const SYSTEM_NEEDS: [&str; 3] = ["instructions", "knowledge", "task"];
const KNOWLEDGE_NEEDS: [&str; 1] = ["passages"];
const PASSAGE_NEEDS: [&str; 2] = ["source", "text"];
const CHAT_SEARCH_NEEDS: [&str; 1] = ["question"];

/// Makes a new agent of the integration `integration` with the role `role`,
/// with the default instructions, limits and parameters of its role, and no
/// knowledge yet.
///
/// Input: `name`, the name typed ("Evaluator ADV"), checked here;
/// `integration`, the id of a stored integration; `role`, the role chosen
/// (`"evaluator"`, `"planner"`, `"chat"`).
/// Output: the id of the new agent (`3`); `DomainError::Refused` for a name
/// refused or taken or a role unknown, an error of the database for an
/// integration missing.
///
/// Called by: `panels/views/agents/agent.rs`: `agent_create`.
/// Calls: `objects/ai/integration/integration_store.rs`: `read_integration`;
/// `objects/ai/agent_roles/agent_roles_rules.rs`: `new_role`; [`checked`]; `objects/ai/agent/agent_store.rs`:
/// `create_agent`.
pub fn new_agent(db: &UserDatabase, name: &str, integration: i64, role: &str) -> Result<i64, DomainError> {
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
        max_answer_tokens: NEW_ANSWER_TOKENS,
        system_template: NEW_SYSTEM_TEMPLATE.to_owned(),
        knowledge_template: NEW_KNOWLEDGE_TEMPLATE.to_owned(),
        passage_template: NEW_PASSAGE_TEMPLATE.to_owned(),
        chat_search: NEW_CHAT_SEARCH.to_owned(),
        role: new_role(role)?,
    };

    Ok(create_agent(db, &checked(db, &agent)?)?)
}

/// Writes the agent `agent.id` anew: its name, integration, instructions,
/// knowledge bases, glossary, limits and the parameters of its role.
///
/// Input: `agent`, the agent as changed, its id that of a stored agent, its
/// role that of the stored one.
/// Output: nothing; `DomainError::Refused` for another role or what
/// [`checked`] refuses, an error of the database for an agent missing.
///
/// Called by: `panels/views/agents/agent.rs`: `agent_save`.
/// Calls: `objects/ai/agent/agent_store.rs`: `read_agent`, `update_agent`;
/// `objects/ai/agent_roles/agent_roles_store.rs`: `role_name`; [`checked`].
pub fn change_agent(db: &UserDatabase, agent: &Agent) -> Result<(), DomainError> {
    let stored = read_agent(db, agent.id)?;

    // Plans and evaluations name the agent for its work (user, 2026-11-02).
    ensure!(
        role_name(&stored.role) == role_name(&agent.role),
        DomainError::Refused("the role of an agent never changes: make a new agent".to_owned())
    );

    Ok(update_agent(db, &checked(db, agent)?)?)
}

/// The glossary with its text trimmed, the rows left empty taken out.
///
/// Input: `glossary`, the glossary as typed.
/// Output: the rows kept (`[GlossaryEntry { term: "TSF", meaning: "TOE
/// Security Functionality" }]`); `DomainError::Refused` when there are more
/// than `MAX_TERMS`, a term has no meaning or a meaning no term, or a text is
/// over [`MAX_TERM`].
///
/// Called by: [`checked`].
fn checked_glossary(glossary: &[GlossaryEntry]) -> Result<Vec<GlossaryEntry>, DomainError> {
    ensure!(glossary.len() <= MAX_TERMS, DomainError::Refused(format!("more than {MAX_TERMS} terms")));

    let mut kept = Vec::with_capacity(glossary.len());

    for entry in glossary {
        let (term, meaning) = (entry.term.trim(), entry.meaning.trim());

        if term.is_empty() && meaning.is_empty() {
            continue;
        }

        ensure!(
            !term.is_empty() && !meaning.is_empty(),
            DomainError::Refused(format!("the term \"{term}{meaning}\" needs both the term and its meaning"))
        );
        ensure!(
            term.chars().count() <= MAX_TERM && meaning.chars().count() <= MAX_TERM,
            DomainError::Refused(format!("a term or a meaning over {MAX_TERM} characters"))
        );
        kept.push(GlossaryEntry { term: term.to_owned(), meaning: meaning.to_owned() });
    }

    Ok(kept)
}

/// The agent with its text trimmed, checked: a name, an integration and
/// knowledge bases that exist, each base once, limits in their ranges, the
/// parameters of its role.
///
/// Input: `agent`, the agent as given by the page or made by [`new_agent`].
/// Output: the agent to write, its bases each once; `DomainError::Refused`
/// for a limit out of its range or text over its limit, an error of the
/// database for an integration or a base missing.
///
/// Called by: [`new_agent`], [`change_agent`].
/// Calls: `objects/text.rs`: `checked_text`; `objects/ai/integration/integration_store.rs`: `read_integration`;
/// `objects/ai/rag/rag_store.rs`: `read_knowledge_base`; [`checked_glossary`];
/// `objects/ai/agent_roles/agent_roles_rules.rs`: `checked_role`.
fn checked(db: &UserDatabase, agent: &Agent) -> Result<Agent, DomainError> {
    let name = checked_text(&agent.name, "the name of the agent", MAX_NAME)?;
    let passages = usize::try_from(agent.max_passages)
        .map_err(|_| DomainError::Refused(format!("the passages of a question: from 1 to {MAX_FOUND}")))?;

    ensure!(
        agent.instructions.chars().count() <= MAX_INSTRUCTIONS,
        DomainError::Refused(format!("instructions over {MAX_INSTRUCTIONS} characters"))
    );
    ensure!(
        (1..=MAX_FOUND).contains(&passages),
        DomainError::Refused(format!("the passages of a question: from 1 to {MAX_FOUND}"))
    );
    ensure!(
        (MIN_CHARACTERS..=MAX_CHARACTERS).contains(&agent.max_characters),
        DomainError::Refused(format!("the characters of a question: from {MIN_CHARACTERS} to {MAX_CHARACTERS}"))
    );
    ensure!(agent.knowledge_bases.len() <= MAX_BASES, DomainError::Refused("too many knowledge bases".to_owned()));
    ensure!(
        (MIN_ANSWER_TOKENS..=MAX_ANSWER_TOKENS).contains(&agent.max_answer_tokens),
        DomainError::Refused(format!("the longest answer: from {MIN_ANSWER_TOKENS} to {MAX_ANSWER_TOKENS} tokens"))
    );
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
        name,
        integration: agent.integration,
        instructions: agent.instructions.trim().to_owned(),
        knowledge_bases: bases,
        glossary: checked_glossary(&agent.glossary)?,
        max_passages: agent.max_passages,
        max_characters: agent.max_characters,
        max_answer_tokens: agent.max_answer_tokens,
        system_template: checked_text(&agent.system_template, "the template of the message", MAX_TEMPLATE)?,
        knowledge_template: checked_text(&agent.knowledge_template, "the template of the knowledge", MAX_TEMPLATE)?,
        passage_template: checked_text(&agent.passage_template, "the template of a passage", MAX_TEMPLATE)?,
        chat_search: checked_text(&agent.chat_search, "the search of a chat", MAX_TEMPLATE)?,
        role: checked_role(&agent.role)?,
    })
}

/// The warnings of the templates of the agent `agent` and of its role: each
/// placeholder a message or a question needs and its template does not hold
/// (user, 2026-11-02: saved all the same, shown in the editor).
///
/// Input: `agent`, a stored agent.
/// Output: the warnings (`["the template of the message has no {task}"]`);
/// none when every template holds what it needs.
///
/// Called by: `panels/views/agents/agent.rs`: `agent_get`.
/// Calls: `objects/ai/agent_roles/agent_roles_rules.rs`: `template_warnings`, `role_warnings`.
pub fn agent_warnings(agent: &Agent) -> Vec<String> {
    let mut warnings = template_warnings("the template of the message", &agent.system_template, &SYSTEM_NEEDS);

    warnings.extend(template_warnings("the template of the knowledge", &agent.knowledge_template, &KNOWLEDGE_NEEDS));
    warnings.extend(template_warnings("the template of a passage", &agent.passage_template, &PASSAGE_NEEDS));
    warnings.extend(template_warnings("the search of a chat", &agent.chat_search, &CHAT_SEARCH_NEEDS));
    warnings.extend(role_warnings(&agent.role));

    warnings
}

#[cfg(test)]
mod tests {
    //! Tests of the agents: made, checked, changed.

    use super::*;

    use crate::objects::ai::agent_roles::agent_roles_model::AgentRole;
    use crate::objects::ai::integration::integration_store::create_integration;
    use crate::objects::ai::rag::rag_store::create_knowledge_base;
    use crate::test_support::database::{database, integration, knowledge_base};

    #[test]
    fn an_agent_is_made_checked_and_changed() {
        let (_directory, db) = database();
        let local = create_integration(&db, &integration()).unwrap();
        let id = new_agent(&db, "  agent_WU_1-1 ", local, "evaluator").unwrap();
        let base = create_knowledge_base(&db, &knowledge_base("CEM")).unwrap();
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
            Agent { system_template: " ".to_owned(), ..stored.clone() },
            Agent { glossary: vec![GlossaryEntry { term: "ST".to_owned(), meaning: String::new() }], ..stored.clone() },
        ];

        for agent in &refused {
            assert!(matches!(change_agent(&db, agent), Err(DomainError::Refused(_))));
        }

        assert!(matches!(
            change_agent(&db, &Agent { knowledge_bases: vec![base + 1], ..stored }),
            Err(DomainError::Db(_))
        ));
        assert!(new_agent(&db, "Another", local + 1, "evaluator").is_err());
    }

    #[test]
    fn a_template_of_an_agent_without_its_placeholder_is_warned_about() {
        let (_directory, db) = database();
        let local = create_integration(&db, &integration()).unwrap();
        let id = new_agent(&db, "Evaluator", local, "evaluator").unwrap();
        let stored = read_agent(&db, id).unwrap();

        assert!(agent_warnings(&stored).is_empty());

        let changed = Agent { system_template: "{instructions}\n\n{knowledge}".to_owned(), ..stored };

        change_agent(&db, &changed).unwrap();
        assert_eq!(agent_warnings(&read_agent(&db, id).unwrap()), vec!["the template of the message has no {task}"]);
    }

    #[test]
    fn the_role_of_an_agent_is_kept_when_it_is_changed() {
        let (_directory, db) = database();
        let local = create_integration(&db, &integration()).unwrap();
        let id = new_agent(&db, "Planner", local, "planner").unwrap();
        let stored = read_agent(&db, id).unwrap();

        assert!(matches!(stored.role, AgentRole::Planner(_)));
        assert!(matches!(
            change_agent(&db, &Agent { role: new_role("evaluator").unwrap(), ..stored }),
            Err(DomainError::Refused(_))
        ));
        assert!(new_agent(&db, "Auditor", local, "auditor").is_err());
    }
}
