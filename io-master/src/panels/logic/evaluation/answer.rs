//! The answer of the assessment of one work unit (user, 2026-10-30), read
//! and written into the work unit as the results of the AI.
//!
//! ```text
//! read_assessment ──► Assessment: the verdict, the findings, analysis, vulnerability notes, notes, the quotes
//! apply_assessment:
//!   the verdict      one of the plan, else the answer is not taken
//!   each quote       of a document of the work unit, found in its text word for word (the spaces passed over):
//!                    kept as a Quote; otherwise written into the notes
//!   the dependencies the verdict is the worst of its own and theirs (Fail, Inconclusive, Pass);
//!                    each one that did not pass written into the findings
//!   ──► update_work_unit, author Ai
//! ```
//!
//! A quote is only a fragment of a document of the product (user,
//! 2026-10-30): the contribution of the manufacturer is assessed; whether it
//! stands in the document is checked here, never taken from the AI (P5).

use serde_json::Value;

use crate::db::models::evaluation::evaluation_plan::read_evaluation_plan;
use crate::db::models::evaluation::product_document::{read_document_text, read_product_document};
use crate::db::models::evaluation::work_unit::{MAX_QUOTES, read_work_unit, update_work_unit};
use crate::db::storage::user_database::UserDatabase;
use crate::model::evaluation::work_unit::{Author, Quote, Verdict, WorkUnit};
use crate::panels::logic::evaluation::error::EvaluationError;
use crate::panels::logic::evaluation::question::verdict_name;

/// The longest text of a result taken from an answer, in characters.
const MAX_RESULT: usize = 100_000;

/// The longest quote taken from an answer, in characters.
const MAX_QUOTE: usize = 5000;

/// What an answer proposes, before it is checked against the work unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assessment {
    pub verdict: String,
    pub findings: String,
    pub analysis: String,
    pub vulnerability_notes: String,
    pub notes: String,
    /// The name of a document and the text quoted from it.
    pub quotes: Vec<(String, String)>,
}

/// The JSON object of an answer (a fence of Markdown or words around it are
/// passed over); refused when the answer holds none.
///
/// Called by: [`read_assessment`].
fn answer_object(answer: &str) -> Result<Value, EvaluationError> {
    let (Some(start), Some(end)) = (answer.find('{'), answer.rfind('}')) else {
        return Err(EvaluationError::Refused("the answer holds no JSON object".to_owned()));
    };

    serde_json::from_str(answer.get(start..=end).unwrap_or_default())
        .map_err(|error| EvaluationError::Refused(format!("the answer is no JSON object: {error}")))
}

/// The text of the field `field` of the answer, trimmed; refused over
/// [`MAX_RESULT`] characters.
///
/// Called by: [`read_assessment`].
fn text_of(value: &Value, field: &str) -> Result<String, EvaluationError> {
    let text = value.get(field).and_then(Value::as_str).unwrap_or_default().trim();

    ensure!(
        text.chars().count() <= MAX_RESULT,
        EvaluationError::Refused(format!("the {field} of the answer over {MAX_RESULT} characters"))
    );

    Ok(text.to_owned())
}

/// The assessment an answer proposes; refused when it holds no JSON object.
///
/// Called by: `run.rs` (each answer).
/// Calls: [`answer_object`], [`text_of`].
pub fn read_assessment(answer: &str) -> Result<Assessment, EvaluationError> {
    let value = answer_object(answer)?;
    let mut quotes = Vec::new();

    if let Some(items) = value.get("quotes").and_then(Value::as_array) {
        for item in items.iter().take(MAX_QUOTES) {
            let document = item.get("document").and_then(Value::as_str).unwrap_or_default().trim();
            let text = item.get("text").and_then(Value::as_str).unwrap_or_default().trim();

            if !text.is_empty() {
                quotes.push((document.to_owned(), text.chars().take(MAX_QUOTE).collect()));
            }
        }
    }

    Ok(Assessment {
        verdict: text_of(&value, "verdict")?.to_lowercase(),
        findings: text_of(&value, "findings")?,
        analysis: text_of(&value, "analysis")?,
        vulnerability_notes: text_of(&value, "vulnerability_notes")?,
        notes: text_of(&value, "notes")?,
        quotes,
    })
}

/// `text` with every run of spaces, tabs and line ends made one space, so
/// that a quote is found however the lines of its document were broken.
///
/// Called by: [`checked_quotes`].
fn spaced(text: &str) -> String {
    text.split_whitespace().collect::<Vec<&str>>().join(" ")
}

/// The quotes `proposed` that stand word for word in the documents of the
/// work unit `unit`, as quotes; and a note for each that does not.
///
/// Called by: [`apply_assessment`], [`check_quote`].
/// Calls: `read_product_document`, `read_document_text`, [`spaced`].
fn checked_quotes(
    db: &UserDatabase,
    unit: &WorkUnit,
    proposed: &[(String, String)],
) -> Result<(Vec<Quote>, Vec<String>), EvaluationError> {
    let mut texts = Vec::with_capacity(unit.input_product_documents.len());

    for id in &unit.input_product_documents {
        texts.push((*id, read_product_document(db, *id)?.name, spaced(&read_document_text(db, *id)?)));
    }

    let mut quotes = Vec::with_capacity(proposed.len());
    let mut notes = Vec::new();

    for (name, text) in proposed {
        let wanted = spaced(text);
        let mut found = None;

        for (id, document, content) in &texts {
            if document.trim().eq_ignore_ascii_case(name) && content.contains(&wanted) {
                found = Some(*id);
            }
        }

        match found {
            Some(product_document) => quotes.push(Quote { product_document, text: text.clone() }),
            None => notes.push(format!("The AI quoted a text not found in {name}: \"{text}\"")),
        }
    }

    Ok((quotes, notes))
}

/// The verdict of its name, if the plan knows it.
///
/// Called by: [`apply_assessment`].
/// Calls: [`verdict_name`].
fn verdict_of(name: &str, possible: &[Verdict]) -> Option<Verdict> {
    for verdict in possible {
        if verdict_name(*verdict) == name {
            return Some(*verdict);
        }
    }

    None
}

/// How bad a verdict is: `Fail` the worst, `Pass` the best.
///
/// Called by: [`with_dependencies`].
fn badness(verdict: Verdict) -> u8 {
    match verdict {
        Verdict::Fail => 2,
        Verdict::Inconclusive => 1,
        Verdict::Pass => 0,
    }
}

/// The verdict `own` of the work unit `unit` made the worst of its own and
/// those of its dependencies (user, 2026-10-28, as Common Criteria asks), and
/// the lines of the findings for each dependency that did not pass or is not
/// assessed.
///
/// Called by: [`apply_assessment`].
/// Calls: `read_work_unit`, [`badness`].
fn with_dependencies(
    db: &UserDatabase,
    unit: &WorkUnit,
    own: Verdict,
) -> Result<(Verdict, Vec<String>), EvaluationError> {
    let mut verdict = own;
    let mut lines = Vec::new();

    for id in &unit.dependencies {
        let dependency = read_work_unit(db, *id)?;

        match dependency.verdict {
            Some(Verdict::Pass) => {}
            Some(other) => {
                lines.push(format!(
                    "{}: its dependency {} {}",
                    verdict_name(other),
                    dependency.label,
                    match other {
                        Verdict::Fail => "failed",
                        _ => "is inconclusive",
                    }
                ));

                if badness(other) > badness(verdict) {
                    verdict = other;
                }
            }
            None => lines.push(format!("its dependency {} is not assessed", dependency.label)),
        }
    }

    Ok((verdict, lines))
}

/// `first` and the lines `more`, parted by a line end when both hold text.
///
/// Called by: [`apply_assessment`].
fn joined(first: &str, more: &[String]) -> String {
    let mut text = first.to_owned();

    for line in more {
        if !text.is_empty() {
            text.push('\n');
        }

        text.push_str(line);
    }

    text
}

/// Writes the assessment `assessment` into the work unit `unit` of the
/// evaluation `evaluation`, as the results of the AI; gives the notes of
/// what was not taken (a quote not found). Refused when the verdict is none
/// of the plan.
///
/// Called by: `run.rs` (each answer).
/// Calls: `read_evaluation_plan`, `read_work_unit`, [`verdict_of`],
/// [`checked_quotes`], [`with_dependencies`], [`joined`], `update_work_unit`.
pub fn apply_assessment(
    db: &UserDatabase,
    evaluation: i64,
    unit: i64,
    assessment: &Assessment,
) -> Result<Vec<String>, EvaluationError> {
    let stored = read_evaluation_plan(db, evaluation)?;
    let this = read_work_unit(db, unit)?;

    ensure!(
        stored.work_units.contains(&unit),
        EvaluationError::Refused("the work unit is not one of the evaluation".to_owned())
    );

    let Some(own) = verdict_of(&assessment.verdict, &stored.possible_verdicts) else {
        return Err(EvaluationError::Refused(format!("the verdict \"{}\" is none of the plan", assessment.verdict)));
    };
    let (quotes, left_out) = checked_quotes(db, &this, &assessment.quotes)?;
    let (verdict, dependency_lines) = with_dependencies(db, &this, own)?;
    let assessed = WorkUnit {
        findings: joined(&assessment.findings, &dependency_lines),
        analysis: assessment.analysis.clone(),
        vulnerability_notes: assessment.vulnerability_notes.clone(),
        notes: joined(&assessment.notes, &left_out),
        quotes,
        verdict: Some(verdict),
        author: Some(Author::Ai),
        ..this
    };

    update_work_unit(db, &assessed)?;

    Ok(left_out)
}

/// Whether the quote `text` of the document `document` stands word for word
/// in it, for the work unit `unit`: the check of a quote the evaluator writes
/// (user, 2026-10-30), as of those of the AI.
///
/// Called by: `results.rs` (the results saved by the evaluator).
/// Calls: [`checked_quotes`].
pub fn check_quote(db: &UserDatabase, unit: &WorkUnit, document: &str, text: &str) -> Result<bool, EvaluationError> {
    let (quotes, _) = checked_quotes(db, unit, &[(document.to_owned(), text.to_owned())])?;

    Ok(!quotes.is_empty())
}

#[cfg(test)]
mod tests {
    //! Tests of the answer: read from its JSON, the quotes checked in the
    //! documents, the verdict made the worst with the dependencies.

    use super::*;

    use crate::db::models::evaluation::work_unit::read_work_unit;
    use crate::test_support::database::database;
    use crate::test_support::evaluation::evaluation;

    fn answer(verdict: &str, quotes: &str) -> String {
        format!(
            "Here it is:\n```json\n{{\"verdict\": \"{verdict}\", \"findings\": \"\", \"analysis\": \"The ST names \
             the TOE.\", \"vulnerability_notes\": \"\", \"notes\": \"\", \"quotes\": [{quotes}]}}\n```"
        )
    }

    #[test]
    fn a_quote_is_kept_only_when_it_stands_in_its_document() {
        let (_directory, db) = database();
        let (id, first, _) = evaluation(&db);
        let quotes = "{\"document\": \"ST\", \"text\": \"The TOE is the router R7 with firmware 2.4.1.\"}, \
                      {\"document\": \"ST\", \"text\": \"The TOE is a firewall.\"}, \
                      {\"document\": \"IXIT\", \"text\": \"Its TSF filters packets.\"}";
        let assessment = read_assessment(&answer("Pass", quotes)).unwrap();
        let left_out = apply_assessment(&db, id, first, &assessment).unwrap();
        let stored = read_work_unit(&db, first).unwrap();

        assert_eq!((stored.verdict, stored.author), (Some(Verdict::Pass), Some(Author::Ai)));
        assert_eq!(stored.quotes.len(), 1);
        assert_eq!(stored.quotes[0].text, "The TOE is the router R7 with firmware 2.4.1.");
        assert_eq!(left_out.len(), 2);
        assert!(stored.notes.contains("not found in ST: \"The TOE is a firewall.\""));
        assert!(stored.notes.contains("not found in IXIT"));
        assert!(check_quote(&db, &stored, "st", "Its TSF filters\npackets.").unwrap());
    }

    #[test]
    fn the_verdict_is_the_worst_of_its_own_and_its_dependencies() {
        let (_directory, db) = database();
        let (id, first, second) = evaluation(&db);

        apply_assessment(&db, id, second, &read_assessment(&answer("pass", "")).unwrap()).unwrap();
        assert!(read_work_unit(&db, second).unwrap().findings.contains("ASE_INT.1-1 is not assessed"));

        apply_assessment(&db, id, first, &read_assessment(&answer("fail", "")).unwrap()).unwrap();
        apply_assessment(&db, id, second, &read_assessment(&answer("pass", "")).unwrap()).unwrap();

        let stored = read_work_unit(&db, second).unwrap();

        assert_eq!(stored.verdict, Some(Verdict::Fail));
        assert_eq!(stored.findings, "fail: its dependency ASE_INT.1-1 failed");
    }

    #[test]
    fn an_answer_without_json_or_with_a_verdict_of_no_plan_is_not_taken() {
        let (_directory, db) = database();
        let (id, first, _) = evaluation(&db);

        assert!(matches!(read_assessment("It passes."), Err(EvaluationError::Refused(_))));
        assert!(matches!(
            apply_assessment(&db, id, first, &read_assessment(&answer("inconclusive", "")).unwrap()),
            Err(EvaluationError::Refused(_))
        ));
        assert_eq!(read_work_unit(&db, first).unwrap().verdict, None);
    }
}
