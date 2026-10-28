//! The task `input_chat` of the AI (role *consultation*, step 52 of
//! docs/history.md): the chat of an input of the Evaluation lab. The first
//! message gives the model the instructions, the requirement word for word,
//! what the input expects and the start of its evidence
//! (`evidence_text::excerpt`); then the conversation so far (its last
//! [`MEMORY`] messages) and the question of the evaluator. The answers are
//! advice: they decide nothing.

use crate::model_old::ai::{Speaker, Turn};
use crate::model_old::evaluation::Evidence;
use crate::model_old::task_scheme::BlockFields;

use super::evidence_text::excerpt;

/// The most earlier messages of the chat the model is given.
pub const MEMORY: usize = 20;

/// The instructions when the evaluator has not changed them.
pub const DEFAULT_INSTRUCTIONS: &str = "You help an evaluator of a security evaluation to judge whether the evidence delivered for one input of a requirement is complete and clear enough to be assessed. You do not decide whether the requirement is met: the evaluator does. Answer the questions about the evidence briefly and precisely; when you point to the evidence, say on which page. Say plainly when the evidence shown does not hold what is needed.";

/// The conversation sent to the model: the context, the last [`MEMORY`]
/// messages of `history` and `question`.
///
/// Called by: `views/evaluation_lab/input_chat/chat.rs`.
/// Calls: `excerpt`.
pub fn turns(
    instructions: &str,
    requirement: &str,
    input: &BlockFields,
    evidence: &[Evidence],
    history: &[Turn],
    question: &str,
) -> Vec<Turn> {
    let context = format!(
        "{}\n\nThe requirement:\n{requirement}\nThe input: {}\nThe evidence it expects: {}\nWhat the planner asks to check: {}\n\nThe evidence delivered:\n{}",
        instructions.trim(),
        input.name,
        input.evidence,
        input.instructions,
        if evidence.is_empty() { "(none yet)\n".to_owned() } else { excerpt(evidence) },
    );
    let mut turns = Vec::with_capacity(MEMORY + 3);

    turns.push(Turn { speaker: Speaker::Evaluator, text: context });
    turns.push(Turn { speaker: Speaker::Model, text: "I have read the requirement and the evidence.".to_owned() });

    for turn in history.iter().skip(history.len().saturating_sub(MEMORY)) {
        turns.push(turn.clone());
    }

    turns.push(Turn { speaker: Speaker::Evaluator, text: question.to_owned() });

    turns
}

#[cfg(test)]
mod tests {
    //! Unit tests of `input_chat.rs`.

    use super::*;

    #[test]
    fn the_chat_starts_with_the_context_and_keeps_its_last_messages() {
        let mut input = BlockFields::empty();
        input.name = "Roles document".to_owned();
        let history: Vec<Turn> =
            (0..30).map(|index| Turn { speaker: Speaker::Evaluator, text: format!("message {index}") }).collect();

        let turns = turns("Help.", "1.1.1: Roles\n", &input, &[], &history, "Is it enough?");

        assert!(turns[0].text.contains("The input: Roles document"));
        assert!(turns[0].text.contains("(none yet)"));
        assert_eq!(turns.len(), 2 + MEMORY + 1);
        assert_eq!(turns[2].text, "message 10");
        assert_eq!(turns.last().unwrap().text, "Is it enough?");
    }
}
