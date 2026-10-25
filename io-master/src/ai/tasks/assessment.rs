//! The task `assessment` of the AI (role *formal analysis*, step 52 of
//! docs/history.md): the agent of a requirement. The question holds the
//! requirement word for word, what the planner asks to check, and **only
//! the fragments the evaluator accepted**, by input (decisions of the user,
//! 2026-10-17); the answer gives the four results and a verdict proposed,
//! with its reason. The evaluator corrects them and sets the verdict (P1).

use crate::model::evaluation::{Results, Verdict};

use super::json::{json_object, text_of};

/// The instructions when the evaluator has not changed them.
pub const DEFAULT_INSTRUCTIONS: &str = r#"You are the agent of one requirement of a security evaluation. Below are the requirement word for word (where it stands in its catalog, its parameters and the requirements it links to), what the planner asks to check, its related elements (for example the work units of the methodology), and the fragments of the evidence the evaluator accepted, by input: exact quotes with their pages, and notes. The requirement and its related elements tell you what to assess; they are not evidence. Assess the requirement on the accepted fragments only; do not assume what they do not say.

Write the four results of the evaluation:
- "findings": what the fragments show, with the quotes and pages they come from;
- "analysis": how the findings relate to every part of the requirement;
- "assessment": your assessment and the verdict you propose, with its reasons;
- "vulnerabilities": the weaknesses that may follow from the solution, and notes for the evaluator.
Propose a verdict: "met", "not_met" or "inconclusive" (when the fragments do not decide it). The evaluator decides.

Answer with one JSON object and nothing else, in this form:
{"findings": "...", "analysis": "...", "assessment": "...", "vulnerabilities": "...", "verdict": "met", "reason": "..."}"#;

/// The message to the model: the instructions, the requirement, what the
/// planner asks to check, its related elements (`related`, step 57.1; no
/// section when empty) and the fragments (`fragments`: the text that lists
/// them by input).
///
/// Called by: `views/evaluation_lab/assessment/propose.rs`.
pub fn prompt(instructions: &str, requirement: &str, checks: &str, related: &str, fragments: &str) -> String {
    let checks = if checks.trim().is_empty() { "(nothing more)" } else { checks.trim() };
    let fragments = if fragments.trim().is_empty() { "(no fragment accepted)\n" } else { fragments };

    let related = if related.trim().is_empty() {
        String::new()
    } else {
        format!(
            "\n\nThe related elements of the requirement, word for word (to understand it; not evidence):\n{related}"
        )
    };

    format!(
        "{}\n\nThe requirement:\n{requirement}\nWhat the planner asks to check: {checks}{related}\n\nThe accepted fragments:\n{fragments}",
        instructions.trim()
    )
}

/// Reads the answer of the model: the four results, the verdict proposed
/// (none when the model gave none the program knows) and the reason.
///
/// Called by: `views/evaluation_lab/assessment/propose.rs`.
pub fn read(answer: &str) -> Result<(Results, String), String> {
    let value = json_object(answer).ok_or("the answer holds no JSON object")?;
    let results = Results {
        findings: text_of(&value, "findings"),
        analysis: text_of(&value, "analysis"),
        assessment: text_of(&value, "assessment"),
        vulnerabilities: text_of(&value, "vulnerabilities"),
        verdict: Verdict::from_name(&text_of(&value, "verdict")),
    };

    if results.findings.is_empty() && results.analysis.is_empty() && results.assessment.is_empty() {
        return Err("the answer holds no result".to_owned());
    }

    Ok((results, text_of(&value, "reason")))
}

#[cfg(test)]
mod tests {
    //! Unit tests of `assessment.rs`.

    use super::*;

    #[test]
    fn the_question_holds_only_what_it_is_given() {
        let text = prompt("Assess.", "1.1.1: Roles\n", " ", "", "");

        assert!(text.contains("(nothing more)"));
        assert!(text.contains("(no fragment accepted)"));
        assert!(!text.contains("related elements"));
    }

    #[test]
    fn the_related_elements_have_a_section_of_their_own() {
        let text = prompt("Assess.", "ADV_FSP.4\n", "", "ADV_FSP.4-1 The evaluator shall examine\n", "- \"quote\"\n");

        let related = text.find("The related elements of the requirement").unwrap();
        assert!(related < text.find("The accepted fragments").unwrap());
        assert!(text.contains("ADV_FSP.4-1 The evaluator shall examine"));
    }

    #[test]
    fn the_four_results_and_the_verdict_are_read() {
        let answer = r#"{"findings": "F", "analysis": "A", "assessment": "S", "vulnerabilities": "V", "verdict": "not_met", "reason": "R"}"#;

        let (results, reason) = read(answer).unwrap();

        assert_eq!((results.findings.as_str(), results.verdict, reason.as_str()), ("F", Some(Verdict::NotMet), "R"));
        assert_eq!(read(r#"{"findings": "F", "verdict": "pass"}"#).unwrap().0.verdict, None);
        assert!(read(r#"{"verdict": "met"}"#).is_err());
    }
}
