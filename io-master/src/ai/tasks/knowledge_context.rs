//! The context of the standard an agent with knowledge adds to a question
//! of the AI (steps 57.4 and 57.6 of docs/todo.md): the name and the
//! instructions of the agent and its passages, numbered, put after the
//! question in a section of their own. The passages tell the model what the
//! standard says; they are never evidence.

use crate::model::ai::Turn;
use crate::model::knowledge::Hit;

use super::knowledge_answer::passages_text;

/// The context of agent `name` with its `instructions` and `passages`;
/// empty when there is no passage.
///
/// Called by: `components/knowledge_agent/find.rs`, `views/evaluation_lab/assessment/context.rs`.
/// Calls: `passages_text`.
pub fn text(name: &str, instructions: &str, passages: &[Hit]) -> String {
    if passages.is_empty() {
        return String::new();
    }

    format!("The agent \"{name}\": {}\n\n{}", instructions.trim(), passages_text(passages))
}

/// `question` with the context `context` after it; the question alone when
/// the context is empty.
///
/// Called by: the works of the AI that take an agent (steps 57.6).
pub fn appended(question: String, context: &str) -> String {
    if context.trim().is_empty() {
        return question;
    }

    format!(
        "{question}\n\nThe context of the standard, from the knowledge of an agent (to understand the question; not evidence):\n{context}"
    )
}

/// The conversation `turns` with the context `context` after its last
/// message (the question now asked).
///
/// Called by: the works of the AI that send a conversation (the chats, the conversion of a catalog).
/// Calls: [`appended`].
pub fn into_last(turns: &mut [Turn], context: &str) {
    if let Some(last) = turns.last_mut() {
        last.text = appended(std::mem::take(&mut last.text), context);
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests of `knowledge_context.rs`.

    use super::*;
    use crate::model::ai::Speaker;
    use crate::model::knowledge::Passage;

    fn hit() -> Hit {
        Hit {
            id: 1,
            source: "CC Part 1".to_owned(),
            passage: Passage { page: Some(3), label: String::new(), text: "The TSF is ...".to_owned() },
            score: -1.0,
        }
    }

    #[test]
    fn the_context_follows_the_question_in_a_section_of_its_own() {
        let context = text("CC agent", " You know CC. ", &[hit()]);

        assert!(context.starts_with("The agent \"CC agent\": You know CC.\n\n[1] CC Part 1 · page 3\nThe TSF is ..."));
        assert!(appended("Q?".to_owned(), &context).starts_with("Q?\n\nThe context of the standard"));
        assert_eq!(appended("Q?".to_owned(), ""), "Q?");
        assert_eq!(text("CC agent", "", &[]), "");
    }

    #[test]
    fn the_context_goes_after_the_last_message() {
        let mut turns = vec![
            Turn { speaker: Speaker::Evaluator, text: "First.".to_owned() },
            Turn { speaker: Speaker::Model, text: "Answer.".to_owned() },
            Turn { speaker: Speaker::Evaluator, text: "Now?".to_owned() },
        ];

        into_last(&mut turns, "[1] passage");

        assert_eq!(turns[0].text, "First.");
        assert!(turns[2].text.starts_with("Now?\n\nThe context of the standard"));
    }
}
