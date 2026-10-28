//! The task `output_draft` of the AI (role *summary*, step 52 of
//! docs/history.md): the text of one output of a task, written from the four
//! results and the verdict decided by the evaluator, as the output asks
//! (its name, format, target and instructions: the template of the
//! laboratory). The evaluator corrects and accepts it.

use crate::model_old::evaluation::Results;
use crate::model_old::task_scheme::{BlockFields, OutputFormat};

/// The instructions when the evaluator has not changed them.
pub const DEFAULT_INSTRUCTIONS: &str = "You write one part of the report of a security evaluation from the results the evaluator accepted. Keep to what the results say: add no finding, no verdict and no quote of your own. Follow what the output asks (its target and instructions). Answer with the text of the part only, nothing before or after it.";

/// The message to the model: the instructions, the output, the requirement
/// (`label`, `title`) and its results with the verdict.
///
/// Called by: `views/evaluation_lab/outputs/draft.rs`.
pub fn prompt(instructions: &str, output: &BlockFields, label: &str, title: &str, results: &Results) -> String {
    let form = match output.format {
        OutputFormat::Text => "plain text",
        OutputFormat::Latex => "LaTeX (the body only: no preamble, no \\begin{document})",
    };
    let verdict = results.verdict.map_or("not set", |verdict| verdict.name());

    format!(
        "{}\n\nThe output: {}\nIts form: {form}\nWhere it goes: {}\nWhat it asks: {}\n\nThe requirement: {label} {title}\nThe verdict of the evaluator: {verdict}\n\nFindings:\n{}\n\nAnalysis:\n{}\n\nAssessment and verdict proposition:\n{}\n\nVulnerabilities and notes:\n{}\n",
        instructions.trim(),
        output.name,
        output.target,
        output.instructions,
        results.findings,
        results.analysis,
        results.assessment,
        results.vulnerabilities,
    )
}

/// The text of the answer: the answer without spaces around it and without
/// a fence of code a model may put around it.
///
/// Called by: `views/evaluation_lab/outputs/draft.rs`.
pub fn read(answer: &str) -> String {
    let trimmed = answer.trim();

    let Some(rest) = trimmed.strip_prefix("```") else {
        return trimmed.to_owned();
    };

    // The language named after the fence, up to the end of its line.
    let body = rest.split_once('\n').map_or("", |(_, body)| body);

    body.strip_suffix("```").unwrap_or(body).trim().to_owned()
}

#[cfg(test)]
mod tests {
    //! Unit tests of `output_draft.rs`.

    use super::*;

    use crate::model_old::evaluation::Verdict;

    #[test]
    fn the_question_holds_the_output_and_the_results() {
        let mut output = BlockFields::empty();
        output.name = "pETR".to_owned();
        output.format = OutputFormat::Latex;
        let mut results = Results::empty();
        results.findings = "All roles have owners.".to_owned();
        results.verdict = Some(Verdict::Met);

        let text = prompt("Write.", &output, "1.1.1", "Roles", &results);

        assert!(text.contains("The output: pETR"));
        assert!(text.contains("LaTeX"));
        assert!(text.contains("The verdict of the evaluator: met"));
        assert!(text.contains("All roles have owners."));
    }

    #[test]
    fn a_fence_of_code_is_taken_off() {
        assert_eq!(read("```latex\n\\section{Roles}\n```"), "\\section{Roles}");
        assert_eq!(read("  Plain text. "), "Plain text.");
    }
}
