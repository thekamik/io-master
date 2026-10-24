//! The requirements given to the AI for a choice (step 51.4 of
//! docs/todo.md): the tasks of a project (`requirement_choice.rs`) and the
//! related elements of a task (`related_elements.rs`). Each requirement is
//! shown word for word under a short reference ("r1", "r2", ...), never its
//! id alone, since two catalogs may use one id; the list is cut into parts
//! of a bounded size, each asked on its own; the answer names the
//! references chosen, each with its reason.

use serde_json::Value;

use crate::model::oscal::catalog::Control;
use crate::model::oscal::catalog::tree::flat_parts;

use super::json::{json_object, text_of};

/// The most characters of the requirements in one question.
pub const PART_CHARS: usize = 24_000;

/// The most requirements one work reads: a few catalogs of some hundreds.
pub const MAX_CANDIDATES: usize = 5000;

/// A requirement offered to the AI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub catalog_uuid: String,
    pub id: String,
    /// The identifier as the standard writes it (else the id).
    pub label: String,
    pub title: String,
    /// All its parts, word for word.
    pub text: String,
}

/// The text of a requirement for a question: its label, title and every
/// part word for word, one per line, indented by its level. Fails when the
/// parts nest deeper than a catalog may.
///
/// Called by: [`candidate`], `task_scheme.rs`, `related_elements.rs`.
/// Calls: `flat_parts`.
pub fn requirement_text(label: &str, control: &Control) -> Result<String, String> {
    let parts = flat_parts(&control.parts).map_err(|error| error.to_string())?;
    let mut text = format!("{label}: {}\n", control.title);

    for (level, part) in parts {
        let indent = "  ".repeat(level + 1);
        let title = part.title.as_deref().unwrap_or_default();
        let prose = part.prose.as_deref().unwrap_or_default();

        text.push_str(&format!("{indent}[{}] {title} {prose}\n", part.name));
    }

    Ok(text)
}

/// Control `control` of catalog `catalog_uuid` as a candidate.
///
/// Called by: `task_proposals/propose.rs`, `task_links/propose.rs`.
/// Calls: [`requirement_text`].
pub fn candidate(catalog_uuid: &str, control: &Control) -> Result<Candidate, String> {
    let label = control.label().unwrap_or(&control.id).to_owned();

    Ok(Candidate {
        catalog_uuid: catalog_uuid.to_owned(),
        id: control.id.clone(),
        text: requirement_text(&label, control)?,
        label,
        title: control.title.clone(),
    })
}

/// The parts of `candidates`, as ranges of their indices: each holds at most
/// [`PART_CHARS`] characters of text, or one candidate when it is longer
/// alone (a requirement is never cut).
///
/// Called by: `task_proposals/propose.rs`, `task_links/propose.rs`.
pub fn parts(candidates: &[Candidate]) -> Vec<std::ops::Range<usize>> {
    let mut found = Vec::new();
    let mut start = 0;
    let mut size = 0;

    for (index, candidate) in candidates.iter().enumerate() {
        let length = candidate.text.chars().count();

        if index > start && size + length > PART_CHARS {
            found.push(start..index);
            start = index;
            size = 0;
        }

        size += length;
    }

    if start < candidates.len() {
        found.push(start..candidates.len());
    }

    found
}

/// The candidates of a part as the model reads them, each under its
/// reference.
///
/// Called by: `requirement_choice.rs`, `related_elements.rs`.
pub fn listing(part: &[Candidate]) -> String {
    let mut text = String::new();

    for (index, candidate) in part.iter().enumerate() {
        text.push_str(&format!("=== r{} ===\n{}\n", index + 1, candidate.text));
    }

    text
}

/// The answer form the model is asked for.
pub const ANSWER_FORM: &str = r#"Answer with one JSON object and nothing else, in this form:
{"chosen": [{"ref": "r3", "reason": "..."}]}
If you choose none, answer {"chosen": []}."#;

/// Reads the answer of the model for `part`: the indices of the candidates
/// chosen, each with its reason, once each; a reference of no candidate is
/// dropped with the reason.
///
/// Called by: `task_proposals/propose.rs`, `task_links/propose.rs`.
/// Calls: `json_object`, `text_of`.
pub fn read_choices(answer: &str, part: &[Candidate]) -> Result<(Vec<(usize, String)>, Vec<String>), String> {
    let value = json_object(answer).ok_or("the answer holds no JSON object")?;
    let mut chosen: Vec<(usize, String)> = Vec::new();
    let mut dropped = Vec::new();

    for item in value.get("chosen").and_then(Value::as_array).into_iter().flatten().take(MAX_CANDIDATES) {
        let reference = text_of(item, "ref");
        let index = reference.strip_prefix('r').and_then(|number| number.parse::<usize>().ok());

        let Some(index) = index.filter(|index| *index >= 1 && *index <= part.len()) else {
            dropped.push(format!("\"{reference}\": there is no such requirement"));
            continue;
        };

        if chosen.iter().any(|(earlier, _)| *earlier == index - 1) {
            continue;
        }

        chosen.push((index - 1, text_of(item, "reason")));
    }

    Ok((chosen, dropped))
}

#[cfg(test)]
mod tests {
    //! Unit tests of `candidates.rs`.

    use super::*;

    fn candidate_of(id: &str, length: usize) -> Candidate {
        Candidate {
            catalog_uuid: "c".to_owned(),
            id: id.to_owned(),
            label: id.to_owned(),
            title: id.to_owned(),
            text: "x".repeat(length),
        }
    }

    #[test]
    fn the_parts_hold_a_bounded_text_and_never_cut_a_requirement() {
        let candidates = [
            candidate_of("a", PART_CHARS - 10),
            candidate_of("b", 20),
            candidate_of("c", PART_CHARS * 2),
            candidate_of("d", 5),
        ];

        assert_eq!(parts(&candidates), [0..1, 1..2, 2..3, 3..4]);
        assert_eq!(parts(&[candidate_of("a", 5), candidate_of("b", 5)]), [0..2]);
        assert!(parts(&[]).is_empty());
    }

    #[test]
    fn the_choices_name_candidates_once_and_a_wrong_reference_is_dropped() {
        let part = [candidate_of("a", 5), candidate_of("b", 5)];
        let answer =
            r#"{"chosen": [{"ref": "r2", "reason": "Needed."}, {"ref": "r2"}, {"ref": "r9"}, {"ref": "s1.1"}]}"#;

        let (chosen, dropped) = read_choices(answer, &part).unwrap();

        assert_eq!(chosen, [(1, "Needed.".to_owned())]);
        assert_eq!(dropped.len(), 2);
        assert!(read_choices("none", &part).is_err());
    }

    #[test]
    fn a_listing_shows_every_candidate_under_its_reference() {
        let text = listing(&[candidate_of("a", 3), candidate_of("b", 3)]);

        assert!(text.contains("=== r1 ===\nxxx"));
        assert!(text.contains("=== r2 ==="));
    }
}
