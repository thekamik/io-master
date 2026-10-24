//! The task `evidence_fragments` of the AI (role *text analysis*, step 52 of
//! docs/history.md): the question with the requirement word for word, what an
//! input expects and a part of a piece of its evidence
//! (`evidence_text.rs`); the answer gives the fragments that matter for the
//! requirement: exact quotes with their pages, and a note apart. The
//! program looks for every quote in the evidence (`model/evaluation.rs`,
//! `quote_found`); the evaluator accepts the fragments (decisions of the
//! user, 2026-10-17).

use serde_json::Value;

use crate::model::evaluation::QUOTE_MAX;
use crate::model::task_scheme::BlockFields;

use super::json::{json_object, text_of};

/// The instructions when the evaluator has not changed them.
pub const DEFAULT_INSTRUCTIONS: &str = r#"You help an evaluator of a security evaluation. Below are a requirement word for word, what one of its inputs expects, and pages of a piece of evidence delivered for it, each after a line "=== Page N ===".

Find the passages of the evidence that matter for the requirement. For each, copy the exact text (a sentence or a few, never paraphrased: a quote that is not in the evidence is marked as not found), give the number N of its page, and write in "note" why it matters or what it lacks. The note is your own text and is never shown as a quote. Find nothing rather than something vague.

Answer with one JSON object and nothing else, in this form:
{"fragments": [{"page": 3, "quote": "...", "note": "..."}]}"#;

/// A fragment as the AI proposed it, not checked yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposedFragment {
    pub page: Option<u32>,
    pub quote: String,
    pub note: String,
}

/// The message to the model: the instructions, the requirement, the input
/// and the pages `pages` of evidence `name` (`evidence_text::pages_text`).
///
/// Called by: `views/evaluation_lab/fragments/propose.rs`.
pub fn prompt(instructions: &str, requirement: &str, input: &BlockFields, name: &str, pages: &str) -> String {
    format!(
        "{}\n\nThe requirement:\n{requirement}\nThe input: {}\nThe evidence it expects: {}\n\nThe evidence \"{name}\":\n{pages}",
        instructions.trim(),
        input.name,
        input.evidence,
    )
}

/// Reads the answer of the model: the fragments, at most `max`, each with a
/// quote of at most `QUOTE_MAX` characters; one without a quote is dropped
/// with the reason.
///
/// Called by: `views/evaluation_lab/fragments/propose.rs`.
pub fn read(answer: &str, max: usize) -> Result<(Vec<ProposedFragment>, Vec<String>), String> {
    let value = json_object(answer).ok_or("the answer holds no JSON object")?;
    let mut found = Vec::new();
    let mut dropped = Vec::new();

    for (index, item) in value.get("fragments").and_then(Value::as_array).into_iter().flatten().take(max).enumerate() {
        let quote: String = text_of(item, "quote").chars().take(QUOTE_MAX).collect();
        let note: String = text_of(item, "note").chars().take(QUOTE_MAX).collect();
        let page = item.get("page").and_then(Value::as_u64).and_then(|page| u32::try_from(page).ok());

        if quote.is_empty() {
            dropped.push(format!("fragment {}: no quote", index + 1));
            continue;
        }

        found.push(ProposedFragment { page, quote, note });
    }

    Ok((found, dropped))
}

#[cfg(test)]
mod tests {
    //! Unit tests of `evidence_fragments.rs`.

    use super::*;

    #[test]
    fn the_question_holds_the_requirement_the_input_and_the_pages() {
        let mut input = BlockFields::empty();
        input.name = "Roles document".to_owned();

        let text = prompt("Find.", "1.1.1: Roles\n", &input, "roles.pdf", "=== Page 1 ===\nAnna owns the keys.\n");

        assert!(text.contains("The input: Roles document"));
        assert!(text.contains("The evidence \"roles.pdf\":\n=== Page 1 ==="));
    }

    #[test]
    fn the_fragments_are_read_up_to_their_limit() {
        let answer = r#"{"fragments": [{"page": 2, "quote": "Anna owns the keys.", "note": "An owner."}, {"page": 1, "quote": ""}, {"quote": "x"}]}"#;

        let (found, dropped) = read(answer, 2).unwrap();

        assert_eq!(
            found,
            [ProposedFragment { page: Some(2), quote: "Anna owns the keys.".to_owned(), note: "An owner.".to_owned() }]
        );
        assert_eq!(dropped.len(), 1);
        assert!(read("no", 5).is_err());
    }
}
