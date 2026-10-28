//! The task `knowledge_answer` of the AI (role *consultation*, step 57.3 of
//! docs/todo.md): an agent with knowledge answers a question from the
//! passages its bases gave (`views/knowledge/proving_ground/ask.rs`). The
//! question holds the instructions of the agent, the rules of this task,
//! the passages numbered, each with its source and its page or requirement,
//! and the question; the answer gives its text and its quotes, each naming
//! its passage, which the program looks for there (P5).

use serde_json::Value;

use crate::model_old::knowledge::{Hit, MAX_QUOTES};

use super::json::{json_object, text_of};

/// The longest quote of an answer that is kept, in characters.
const QUOTE_CHARS: usize = 2000;

/// The rules of the task when the evaluator has not changed them; the
/// instructions of the agent stand before them.
pub const DEFAULT_INSTRUCTIONS: &str = r#"Answer the question below from the passages given with it, each numbered [N] with its source and its page or requirement. Say which passages your answer rests on. When the passages do not answer the question, say so; do not fill the gap from memory. Quote only text that stands in a passage, word for word: a quote that is not in its passage is marked as not found.

Answer with one JSON object and nothing else, in this form:
{"answer": "...", "quotes": [{"passage": 1, "quote": "..."}]}"#;

/// The passages `passages` for a question, numbered from 1: `[N] source ·
/// requirement · page N`, then the text.
///
/// Called by: [`prompt`].
pub fn passages_text(passages: &[Hit]) -> String {
    let mut text = String::new();

    for (number, hit) in (1..).zip(passages) {
        let mut place = hit.source.clone();

        if !hit.passage.label.is_empty() {
            place.push_str(&format!(" · {}", hit.passage.label));
        }

        if let Some(page) = hit.passage.page {
            place.push_str(&format!(" · page {page}"));
        }

        text.push_str(&format!("[{number}] {place}\n{}\n\n", hit.passage.text));
    }

    text
}

/// The message to the model: the instructions of the agent, the rules of
/// the task, the passages and the question.
///
/// Called by: `views/knowledge/proving_ground/ask.rs`.
/// Calls: [`passages_text`].
pub fn prompt(agent: &str, rules: &str, passages: &[Hit], question: &str) -> String {
    let passages = if passages.is_empty() { "(no passage found)\n".to_owned() } else { passages_text(passages) };

    format!("{}\n\n{}\n\nThe passages:\n{passages}The question: {}", agent.trim(), rules.trim(), question.trim())
}

/// Reads the answer of the model: its text, and its quotes (at most
/// `MAX_QUOTES`), each with the number of the passage it names (0 when it
/// names none); a quote without text is dropped.
///
/// Called by: `views/knowledge/proving_ground/ask.rs`.
pub fn read(answer: &str) -> Result<(String, Vec<(usize, String)>), String> {
    let value = json_object(answer).ok_or("the answer holds no JSON object")?;
    let text = text_of(&value, "answer");
    let mut quotes = Vec::new();

    ensure!(!text.is_empty(), "the answer holds no text".to_owned());

    for item in value.get("quotes").and_then(Value::as_array).into_iter().flatten().take(MAX_QUOTES) {
        let quote: String = text_of(item, "quote").chars().take(QUOTE_CHARS).collect();
        let passage = item.get("passage").and_then(Value::as_u64).and_then(|number| usize::try_from(number).ok());

        if !quote.is_empty() {
            quotes.push((passage.unwrap_or(0), quote));
        }
    }

    Ok((text, quotes))
}

#[cfg(test)]
mod tests {
    //! Unit tests of `knowledge_answer.rs`.

    use super::*;
    use crate::model_old::knowledge::Passage;

    fn hit(label: &str, page: Option<u32>, text: &str) -> Hit {
        Hit {
            id: 1,
            source: "CC Part 2".to_owned(),
            passage: Passage { page, label: label.to_owned(), text: text.to_owned() },
            score: -2.0,
        }
    }

    #[test]
    fn the_passages_are_numbered_with_their_place() {
        let passages = [hit("FCS_CKM.1", None, "Key generation."), hit("", Some(12), "Audit.")];

        let text = prompt("You know CC.", "Rules.", &passages, " How are keys made? ");

        assert!(text.starts_with("You know CC.\n\nRules.\n\nThe passages:\n[1] CC Part 2 · FCS_CKM.1\nKey generation.\n\n[2] CC Part 2 · page 12\nAudit."));
        assert!(text.ends_with("The question: How are keys made?"));
        assert!(prompt("", "Rules.", &[], "Q").contains("(no passage found)"));
    }

    #[test]
    fn the_answer_and_its_quotes_are_read() {
        let answer = r#"{"answer": "Keys are generated.", "quotes": [{"passage": 1, "quote": "Key generation."}, {"quote": ""}, {"quote": "no number"}]}"#;

        let (text, quotes) = read(answer).unwrap();

        assert_eq!(text, "Keys are generated.");
        assert_eq!(quotes, [(1, "Key generation.".to_owned()), (0, "no number".to_owned())]);
        assert!(read(r#"{"quotes": []}"#).is_err());
        assert!(read("no JSON").is_err());
    }
}
