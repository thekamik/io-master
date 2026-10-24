//! The report of an evaluation (step 52 of docs/history.md): the result of the
//! work of the evaluator, exported out of the program (user, 2026-10-24).
//! One file for every output (by its name and format): the accepted texts
//! of that output, task by task in the order of the package, for the tasks
//! whose verdict makes its condition hold. A text output is written as
//! `.txt`, a LaTeX output as `.tex` (the body only, for the template of the
//! laboratory).

use crate::app::error::AppError;
use crate::db::evaluation as db_evaluation;
use crate::db::user::database::UserDb;
use crate::model::evaluation::output_applies;
use crate::model::task_scheme::{BlockKind, OutputFormat};

/// A file of the report: its name and its text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportFile {
    pub name: String,
    pub format: OutputFormat,
    pub text: String,
}

/// The files of the report of evaluation `evaluation`; empty while no
/// output has an accepted text.
///
/// Called by: `report_export` (`evaluation_lab.rs`).
/// Calls: `read::evaluation`, `read::tasks`, `read::blocks`, `read::results`,
/// `read::output`, `output_applies`, [`section`], [`header`].
pub fn files(db: &UserDb, evaluation: &str) -> Result<Vec<ReportFile>, AppError> {
    let Some(found) = db_evaluation::read::evaluation(db, evaluation)? else {
        return Err(AppError::Invalid("the evaluation does not exist".to_owned()));
    };

    let mut files: Vec<(String, ReportFile)> = Vec::new();

    for task in db_evaluation::read::tasks(db, evaluation)? {
        let results = db_evaluation::read::results(db, evaluation, &task.uuid)?;

        for block in db_evaluation::read::blocks(db, evaluation, &task.uuid)? {
            if block.kind != BlockKind::Output || !output_applies(block.fields.condition, results.verdict) {
                continue;
            }

            let Some(output) = db_evaluation::read::output(db, evaluation, &block.uuid)? else {
                continue;
            };

            if output.text.trim().is_empty() {
                continue;
            }

            let key = format!("{}\u{0}{}", block.fields.name, block.fields.format.name());
            let position = match files.iter().position(|(known, _)| *known == key) {
                Some(position) => position,
                None => {
                    let text = header(&found, &block.fields.name, block.fields.format);
                    files.push((
                        key,
                        ReportFile {
                            name: file_name(&block.fields.name, block.fields.format),
                            format: block.fields.format,
                            text,
                        },
                    ));
                    files.len() - 1
                }
            };

            if let Some((_, file)) = files.get_mut(position) {
                file.text.push_str(&section(&task.label, &task.title, block.fields.format, &output.text));
            }
        }
    }

    let mut found_files = Vec::with_capacity(files.len());

    for (_, file) in files {
        found_files.push(file);
    }

    Ok(found_files)
}

/// The head of a file: what the report is of.
///
/// Called by: [`files`].
fn header(evaluation: &crate::model::evaluation::Evaluation, output: &str, format: OutputFormat) -> String {
    let product = format!("{} {}", evaluation.fields.product, evaluation.fields.product_version);
    let lines = [
        output.to_owned(),
        format!("Project: {}", evaluation.project_title),
        format!("Work package: {} ({})", evaluation.package_name, evaluation.evaluator),
        format!("Product: {}", product.trim()),
    ];
    let mut text = String::new();

    for line in lines {
        match format {
            OutputFormat::Text => text.push_str(&format!("{line}\n")),
            OutputFormat::Latex => text.push_str(&format!("% {line}\n")),
        }
    }

    text.push('\n');
    text
}

/// The part of a file for one task.
///
/// Called by: [`files`].
fn section(label: &str, title: &str, format: OutputFormat, text: &str) -> String {
    match format {
        OutputFormat::Text => format!("== {label} {title} ==\n\n{}\n\n", text.trim()),
        OutputFormat::Latex => format!("% ---- {label} {title} ----\n{}\n\n", text.trim()),
    }
}

/// The name of the file of output `name`: its letters, digits, spaces and
/// `-_.`, the rest as `_`, and its ending.
///
/// Called by: [`files`].
fn file_name(name: &str, format: OutputFormat) -> String {
    let mut safe = String::new();

    for character in name.trim().chars().take(100) {
        safe.push(if character.is_alphanumeric() || "-_ .".contains(character) { character } else { '_' });
    }

    if safe.is_empty() {
        safe.push_str("output");
    }

    match format {
        OutputFormat::Text => format!("{safe}.txt"),
        OutputFormat::Latex => format!("{safe}.tex"),
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests of `export.rs`.

    use super::*;

    use crate::model::evaluation::{Results, Verdict};
    use crate::test_support::db::{lab_evaluation, lab_package};
    use crate::test_support::standards::database;

    #[test]
    fn the_report_holds_the_accepted_texts_of_the_outputs_whose_condition_holds() {
        let test = database();
        let package = lab_package("c");
        db_evaluation::create::evaluation(&test.db, &lab_evaluation(&package), &package).unwrap();
        db_evaluation::update::output_text(&test.db, "e", "o", "\\paragraph{Roles} All roles have owners.").unwrap();
        db_evaluation::update::output_text(&test.db, "e", "n", "An observation.").unwrap();

        let mut results = Results::empty();
        results.verdict = Some(Verdict::Met);
        db_evaluation::update::results(&test.db, "e", "t", &results).unwrap();

        let report = files(&test.db, "e").unwrap();

        assert_eq!(report.len(), 1, "the Observation Report waits for a verdict not met");
        assert_eq!(report[0].name, "pETR.tex");
        assert!(report[0].text.starts_with("% pETR\n% Project: CC"));
        assert!(
            report[0]
                .text
                .contains("% ---- 1.1.1 Information security roles and responsibilities ----\n\\paragraph{Roles}")
        );

        results.verdict = Some(Verdict::NotMet);
        db_evaluation::update::results(&test.db, "e", "t", &results).unwrap();
        assert_eq!(files(&test.db, "e").unwrap().len(), 2);
        assert_eq!(file_name("O/R: x", OutputFormat::Text), "O_R_ x.txt");
    }
}
