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
//! agent_question ──► system_template of the agent: {instructions}, {knowledge}, {task} + the messages ──► Question
//!
//! filled(template, values) ──► each {placeholder} of the template replaced by its value, in one pass
//! ```

use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::ai::agent::agent_model::GlossaryEntry;
use crate::objects::ai::agent::agent_store::read_agent;
use crate::objects::ai::chat::chat_model::Message;
use crate::objects::ai::integration::integration_ai::question::Question;
use crate::objects::ai::rag::rag_store::{MAX_FOUND, read_passages};
use crate::objects::ai_error::AiError;

/// The value of the placeholder that `text` starts with, just after its `{`
/// (`"documents} ..."`), and the text after the placeholder.
///
/// Input: `text`, the rest of a template after a `{`; `values`, the
/// placeholders without braces with their values.
/// Output: the value and the rest (`Some(("ST ...", " ..."))`); `None` when
/// no placeholder of `values` starts there.
///
/// Called by: [`filled`].
fn placeholder_at<'a, 'b, V: AsRef<str>>(text: &'b str, values: &'a [(&str, V)]) -> Option<(&'a str, &'b str)> {
    for (name, value) in values {
        if let Some(rest) = text.strip_prefix(*name)
            && let Some(rest) = rest.strip_prefix('}')
        {
            return Some((value.as_ref(), rest));
        }
    }

    None
}

/// The template `template` of an agent or of its role with each placeholder
/// of `values` (`{documents}`) replaced by its value, in one pass: a
/// placeholder that comes inside a value (a document that writes
/// "{documents}") is never replaced again, and a placeholder the program
/// does not know stays as it is written.
///
/// Input: `template`, a template, bounded when it was written
/// (`agent_store.rs`, `agent_roles_store.rs`); `values`, the placeholders
/// without braces with their values (`[("label", "ADV_ARC.1-1")]`).
/// Output: the text ("Work unit: ADV_ARC.1-1").
///
/// Called by: [`agent_knowledge`], [`agent_question`];
/// `objects/evaluation/work_unit/work_unit_ai/assessment_question.rs`: `dependencies_text`, `documents_text`,
/// `proofs_text`, `task_of`, `question_of`; `objects/evaluation/work_unit/work_unit_ai/proposal.rs`: `question_of`,
/// `parameters_question`, `summary`, `dependencies_question`.
/// Calls: [`placeholder_at`].
pub fn filled<V: AsRef<str>>(template: &str, values: &[(&str, V)]) -> String {
    let mut text = String::with_capacity(template.len());
    let mut rest = template;

    // Each pass takes at least one byte of the template, so it ends within
    // the length of the template.
    for _ in 0..=template.len() {
        let Some((before, after)) = rest.split_once('{') else {
            break;
        };

        text.push_str(before);

        if let Some((value, after_placeholder)) = placeholder_at(after, values) {
            text.push_str(value);
            rest = after_placeholder;
        } else {
            text.push('{');
            rest = after;
        }
    }

    text.push_str(rest);
    text
}

/// The message of an agent with the room of its empty parts taken out: no
/// more than one empty line in a row, none at its ends (an agent without
/// knowledge leaves `{knowledge}` empty).
///
/// Input: `text`, the system template of an agent filled.
/// Output: the text ("You assess ...\n\nAnswer in JSON.").
///
/// Called by: [`agent_question`].
fn tidied(text: &str) -> String {
    let mut tidied = String::with_capacity(text.len());
    let mut empty_before = true;

    for line in text.trim().lines() {
        let empty = line.trim().is_empty();

        if !(empty && empty_before) {
            tidied.push_str(line);
            tidied.push('\n');
        }

        empty_before = empty;
    }

    tidied.trim_end().to_owned()
}

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
/// term the question names is put before it, so that the search finds a text
/// that writes the term out ("TSF" also finds "TOE Security Functionality").
///
/// Input: `question`, the question as asked; `glossary`, the glossary of the
/// agent, checked when the agent was written (`agent_rules.rs`).
/// Output: the meanings, then the question ("TOE Security Functionality What
/// is the TSF?").
///
/// Called by: [`agent_knowledge`].
/// Calls: [`words`], [`names_term`].
fn widened(question: &str, glossary: &[GlossaryEntry]) -> String {
    let asked = words(question);
    let mut widened = String::new();

    // The meanings come first: the search reads a bounded number of words,
    // and the terms of the glossary are what the agent was given them for.
    for entry in glossary {
        if names_term(&asked, &entry.term) {
            widened.push_str(&entry.meaning);
            widened.push(' ');
        }
    }

    widened.push_str(question);
    widened
}

/// The knowledge the agent `agent` gives with the question `question`: the
/// passages of its bases that fit it best, at most its `max_passages`, as
/// many of them, the best first, as fit in its `max_characters`. A passage
/// is never shortened: one that does not fit is left out.
///
/// Input: `agent`, the id of a stored agent; `question`, what the knowledge
/// is looked for with ("ADV_ARC.1-1 The developer shall ...").
/// Output: the passages given, the best first, each written by the
/// `passage_template` of the agent (`["=== CC interpretation 12.pdf ===\nThe
/// TSF shall ..."]`); an error when the agent is missing or damaged or the
/// database fails.
///
/// Called by: [`agent_question`]; `panels/views/agents/agent.rs`:
/// `agent_knowledge_test`.
/// Calls: `objects/ai/agent/agent_store.rs`: `read_agent`; [`widened`]; `objects/ai/rag/rag_store.rs`:
/// `read_passages`; [`filled`].
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

    // The limit counts the text of the passages, not their templates.
    for (source, passage) in found {
        let length = passage.chars().count();

        if characters + length <= limit {
            characters += length;
            given.push(filled(&agent.passage_template, &[("source", source.as_str()), ("text", passage.as_str())]));
        }
    }

    Ok(given)
}

/// The question the agent `agent` asks of its integration: its message laid
/// out by its `system_template` from its instructions, its knowledge that
/// fits `search` (introduced by its `knowledge_template`, none when nothing
/// is found) and the task `task` (none when empty), then the messages
/// `messages`, the last of them the question of the user.
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
/// Calls: `objects/ai/agent/agent_store.rs`: `read_agent`; [`agent_knowledge`], [`filled`], [`tidied`].
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
    let passages = agent_knowledge(db, agent, search)?;
    let knowledge = if passages.is_empty() {
        String::new()
    } else {
        filled(&stored.knowledge_template, &[("passages", passages.join("\n\n"))])
    };
    let values = [("instructions", stored.instructions.as_str()), ("knowledge", knowledge.as_str()), ("task", task)];
    let system = tidied(&filled(&stored.system_template, &values));

    Ok(Question {
        integration: stored.integration,
        agent: Some(agent),
        work_unit,
        system,
        messages,
        max_answer_tokens: stored.max_answer_tokens,
    })
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
    use crate::objects::ai::rag::rag_model::Source;
    use crate::objects::ai::rag::rag_store::{create_knowledge_base, create_source};
    use crate::test_support::database::{agent, database, integration, knowledge_base};

    #[test]
    fn the_knowledge_of_an_agent_is_found_with_its_glossary_and_fits_its_limit() {
        let (_directory, db) = database();
        let local = create_integration(&db, &integration()).unwrap();
        let id = create_agent(&db, &agent(local)).unwrap();
        let base = create_knowledge_base(&db, &knowledge_base("CEM")).unwrap();
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
        assert_eq!(widened("Read the ST.", &glossary), "Security Target Read the ST.");
        assert_eq!(widened("the toe Scope", &glossary), "the boundary the toe Scope");
    }

    #[test]
    fn the_question_of_an_agent_holds_its_instructions_its_task_and_its_knowledge() {
        let (_directory, db) = database();
        let local = create_integration(&db, &integration()).unwrap();
        let id = create_agent(&db, &agent(local)).unwrap();
        let base = create_knowledge_base(&db, &knowledge_base("CEM")).unwrap();
        let text = "The developer shall describe the security architecture.".to_owned();

        create_source(&db, &Source { id: 0, knowledge_base: base, name: "notes".to_owned(), text }).unwrap();
        update_agent(&db, &Agent { id, knowledge_bases: vec![base], ..agent(local) }).unwrap();

        let message = Message { speaker: Speaker::User, text: "Assess it.".to_owned(), time: 0 };
        let question =
            agent_question(&db, id, "Answer in JSON.", "security architecture", Some(3), vec![message]).unwrap();

        assert!(question.system.starts_with("You assess work units of the CEM.\n\nYour knowledge for this question:"));
        assert!(question.system.contains("=== notes ===\nThe developer shall"));
        assert!(question.system.ends_with("\n\nAnswer in JSON."));
        assert_eq!((question.integration, question.agent, question.work_unit), (local, Some(id), Some(3)));

        let chat = agent_question(&db, id, "", "routers", None, question.messages.clone()).unwrap();

        assert_eq!(chat.system, "You assess work units of the CEM.");
        assert!(matches!(agent_question(&db, id, "", "", None, Vec::new()), Err(AiError::NoQuestion)));
    }

    #[test]
    fn a_template_is_filled_in_one_pass() {
        let values = [("name", "ST {text}"), ("text", "The TSF.")];

        assert_eq!(filled("=== {name} ===\n{text}", &values), "=== ST {text} ===\nThe TSF.");
        assert_eq!(filled("{unknown} {name", &values), "{unknown} {name");
        assert_eq!(filled("{{text}}", &values), "{The TSF.}");
        assert_eq!(tidied("\nA\n\n\n\nB\n\n"), "A\n\nB");
    }
}
