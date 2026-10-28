//! The task `related_elements` of the AI (role *text analysis*, step 51.4
//! of docs/history.md): the question with the requirement of a task and a part
//! of the requirements of the other catalogs of its project
//! (`candidates.rs`); the answer, read by `candidates::read_choices`, names
//! the related ones (the work units of the CEM for a requirement of CC
//! Part 3, the test cases of a test specification, ...), each with its
//! reason. The planner accepts or rejects each
//! (`views/evaluation_plan/task_links/`).

use crate::model_old::oscal::catalog::Control;

use super::candidates::{ANSWER_FORM, Candidate, listing, requirement_text};

/// The instructions when the evaluator has not changed them.
pub const DEFAULT_INSTRUCTIONS: &str = r#"You help the planner of a security evaluation to link one requirement of a standard with the elements of other documents that the evaluator needs to evaluate it: the work units of the methodology (for example the CEM for Common Criteria), the test cases of a test specification, the requirements of a Protection Profile that refine it. Below are the requirement word for word and some elements of the other documents, each word for word under its reference ("=== r1 ===").

Choose the elements that belong to the evaluation of this requirement, and give for each the reason. Choose only what is directly related; choose none when nothing is."#;

/// The message to the model: the instructions, the requirement `control`
/// (labelled `label`) word for word and the elements of `part`. Fails when
/// the parts of the control nest deeper than a catalog may.
///
/// Called by: `views/evaluation_plan/task_links/propose.rs`.
/// Calls: `requirement_text`, `listing`.
pub fn prompt(instructions: &str, label: &str, control: &Control, part: &[Candidate]) -> Result<String, String> {
    Ok(format!(
        "{}\n\n{ANSWER_FORM}\n\nThe requirement:\n{}\nThe elements of the other documents:\n{}",
        instructions.trim(),
        requirement_text(label, control)?,
        listing(part)
    ))
}

#[cfg(test)]
mod tests {
    //! Unit tests of `related_elements.rs`.

    use super::*;

    use serde_json::json;

    #[test]
    fn the_question_holds_the_requirement_and_the_elements() {
        let control: Control = serde_json::from_value(json!({
            "id": "adv_fsp.1", "title": "Basic functional specification",
            "parts": [{ "name": "statement", "prose": "The developer shall provide a functional specification." }]
        }))
        .unwrap();
        let unit = Candidate {
            catalog_uuid: "cem".to_owned(),
            id: "adv_fsp.1-1".to_owned(),
            label: "ADV_FSP.1-1".to_owned(),
            title: "Work unit".to_owned(),
            text: "ADV_FSP.1-1: Work unit\n  [statement] The evaluator shall examine.\n".to_owned(),
        };

        let text = prompt("Link.", "ADV_FSP.1", &control, &[unit]).unwrap();

        assert!(text.contains("ADV_FSP.1: Basic functional specification"));
        assert!(text.contains("The developer shall provide a functional specification."));
        assert!(text.contains("=== r1 ===\nADV_FSP.1-1: Work unit"));
    }
}
