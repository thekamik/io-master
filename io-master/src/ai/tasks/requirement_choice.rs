//! The task `requirement_choice` of the AI (role *text analysis*, step 51.4
//! of docs/todo.md): the question with the scope of a master project and a
//! part of the requirements of one of its catalogs (`candidates.rs`); the
//! answer, read by `candidates::read_choices`, names the requirements to
//! evaluate, each with its reason. The planner accepts or rejects each
//! (`views/evaluation_plan/task_proposals/`).

use super::candidates::{ANSWER_FORM, Candidate, listing};

/// The instructions when the evaluator has not changed them.
pub const DEFAULT_INSTRUCTIONS: &str = r#"You help the planner of a security evaluation (Common Criteria, ETSI EN 303 645 or another standard) to choose the requirements of a standard that the evaluation has to cover. Below are the scope of the evaluation and some requirements of a catalog of the standard, each word for word under its reference ("=== r1 ===").

Choose the requirements that the evaluation has to cover for this scope, and give for each the reason (which part of the scope needs it). Leave out the requirements that only explain, define terms or apply to other kinds of products or levels of assurance. When the scope does not decide, choose the requirement and say so in the reason."#;

/// The message to the model: the instructions, the scope of the project,
/// the title of the catalog and the requirements of `part`.
///
/// Called by: `views/evaluation_plan/task_proposals/propose.rs`.
/// Calls: `listing`.
pub fn prompt(instructions: &str, scope: &str, catalog: &str, part: &[Candidate]) -> String {
    let scope = if scope.trim().is_empty() { "(not described)" } else { scope.trim() };

    format!(
        "{}\n\n{ANSWER_FORM}\n\nThe scope of the evaluation:\n{scope}\n\nThe catalog: {catalog}\n\n{}",
        instructions.trim(),
        listing(part)
    )
}

#[cfg(test)]
mod tests {
    //! Unit tests of `requirement_choice.rs`.

    use super::*;

    #[test]
    fn the_question_holds_the_scope_the_catalog_and_the_requirements() {
        let candidate = Candidate {
            catalog_uuid: "c".to_owned(),
            id: "s1.1.1".to_owned(),
            label: "1.1.1".to_owned(),
            title: "Roles".to_owned(),
            text: "1.1.1: Roles\n  [statement] All responsibilities are defined.\n".to_owned(),
        };

        let text = prompt("Choose.", "A smart card.", "Sample catalog", &[candidate]);

        assert!(text.contains("A smart card."));
        assert!(text.contains("The catalog: Sample catalog"));
        assert!(text.contains("=== r1 ===\n1.1.1: Roles"));
        assert!(text.contains("\"chosen\""));
        assert!(prompt("Choose.", " ", "C", &[]).contains("(not described)"));
    }
}
