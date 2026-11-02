//! The answer of the assessment of one work unit (user, 2026-10-30), read
//! and written into the work unit as the results of the AI.
//!
//! ```text
//! read_assessment ──► Assessment: the verdict, the findings, analysis, vulnerability notes, notes, the quotes
//! apply_assessment:
//!   the verdict      one of the plan, else the answer is not taken
//!   each quote       of a document of the work unit, found in its text word for word (the spaces passed over;
//!                    work_unit_rules/results.rs: quotes_found): kept as a Quote; otherwise written into the notes
//!   the dependencies the verdict is the worst of its own and theirs (Fail, Inconclusive, Pass);
//!                    each one that did not pass written into the findings
//!   ──► update_work_unit, author Ai
//! ```
//!
//! A quote is only a fragment of a document of the product (user,
//! 2026-10-30): the contribution of the manufacturer is assessed; whether it
//! stands in the document is checked by the rule of the data, never taken
//! from the AI (P5).

use crate::objects::ai_error::AiError;
use serde_json::Value;

use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::evaluation::evaluation_plan::evaluation_plan_store::read_evaluation_plan;
use crate::objects::evaluation::work_unit::work_unit_ai::assessment_question::verdict_name;
use crate::objects::evaluation::work_unit::work_unit_model::{Author, Verdict, WorkUnit};
use crate::objects::evaluation::work_unit::work_unit_rules::results::quotes_found;
use crate::objects::evaluation::work_unit::work_unit_store::{MAX_QUOTES, read_work_unit, update_work_unit};

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

/// The JSON object of an answer: a fence of Markdown or words around it are
/// passed over.
///
/// Input: `answer`, the text of the answer of the AI, bounded by
/// `MAX_STREAM` (`connection.rs`).
/// Output: the object (`{"verdict": "Pass", ...}`); `AiError::Refused` when
/// the answer holds none.
///
/// Called by: [`read_assessment`].
fn answer_object(answer: &str) -> Result<Value, AiError> {
    let (Some(start), Some(end)) = (answer.find('{'), answer.rfind('}')) else {
        return Err(AiError::Refused("the answer holds no JSON object".to_owned()));
    };

    // A "}" before the first "{" leaves no object between them.
    ensure!(start < end, AiError::Refused("the answer holds no JSON object".to_owned()));

    let object =
        answer.get(start..=end).ok_or_else(|| AiError::Refused("the answer holds no JSON object".to_owned()))?;

    serde_json::from_str(object).map_err(|error| AiError::Refused(format!("the answer is no JSON object: {error}")))
}

/// The text of the field `field` of the answer, trimmed.
///
/// Input: `value`, the object of the answer; `field`, a field ("findings").
/// Output: its text, empty when it has none; `AiError::Refused` over
/// [`MAX_RESULT`] characters.
///
/// Called by: [`read_assessment`].
fn text_of(value: &Value, field: &str) -> Result<String, AiError> {
    let text = value.get(field).and_then(Value::as_str).unwrap_or_default().trim();

    ensure!(
        text.chars().count() <= MAX_RESULT,
        AiError::Refused(format!("the {field} of the answer over {MAX_RESULT} characters"))
    );

    Ok(text.to_owned())
}

/// The assessment an answer proposes, before it is checked against the work
/// unit.
///
/// Input: `answer`, the text of the answer of the AI.
/// Output: the assessment, at most `MAX_QUOTES` quotes, each cut to
/// [`MAX_QUOTE`] characters (`Assessment { verdict: "Pass", .. }`); the
/// refusals of [`answer_object`] and [`text_of`].
///
/// Called by: `objects/evaluation/work_unit/work_unit_ai/assessment.rs`: `assessed`.
/// Calls: [`answer_object`], [`text_of`].
pub fn read_assessment(answer: &str) -> Result<Assessment, AiError> {
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

/// The verdict of its name, if the plan knows it.
///
/// Input: `name`, the verdict the answer gives ("Pass"); `possible`, the
/// verdicts of the plan.
/// Output: the verdict (`Some(Verdict::Pass)`); `None` when the plan does not
/// know it.
///
/// Called by: [`apply_assessment`].
/// Calls: `objects/evaluation/work_unit/work_unit_ai/assessment_question.rs`: `verdict_name`.
fn verdict_of(name: &str, possible: &[Verdict]) -> Option<Verdict> {
    for verdict in possible {
        if verdict_name(*verdict) == name {
            return Some(*verdict);
        }
    }

    None
}

/// How bad a verdict is: `Fail` the worst (2), `Pass` the best (0).
///
/// Input: `verdict`, a verdict.
/// Output: its badness, 0 to 2.
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
/// those of its dependencies (user, 2026-10-28, as Common Criteria asks).
///
/// Input: `unit`, the work unit assessed; `own`, the verdict the answer
/// gives.
/// Output: the verdict and the lines of the findings for each dependency
/// that did not pass or is not assessed (`(Fail, ["Fail: its dependency
/// ADV_ARC.1-1 failed"])`); an error of the database.
///
/// Called by: [`apply_assessment`].
/// Calls: `objects/evaluation/work_unit/work_unit_store.rs`: `read_work_unit`; [`badness`];
/// `objects/evaluation/work_unit/work_unit_ai/assessment_question.rs`: `verdict_name`.
fn with_dependencies(db: &UserDatabase, unit: &WorkUnit, own: Verdict) -> Result<(Verdict, Vec<String>), AiError> {
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

/// `first` and the lines `more`, parted by a line end when both hold text:
/// `joined("A", ["B"])` is "A\nB".
///
/// Input: `first`, a text of the answer; `more`, lines added after it.
/// Output: the text joined.
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
/// evaluation `evaluation`, as the results of the AI.
///
/// Input: `evaluation`, the id of an evaluation in progress; `unit`, the id
/// of one of its work units; `assessment`, from [`read_assessment`].
/// Output: the notes of what was not taken (`["The AI quoted a text not found
/// in ST: ..."]`); `AiError::Refused` for a work unit not of the evaluation
/// or a verdict none of the plan, an error of the database.
///
/// Called by: `objects/evaluation/work_unit/work_unit_ai/assessment.rs`: `assessed`.
/// Calls: `objects/evaluation/evaluation_plan/evaluation_plan_store.rs`: `read_evaluation_plan`;
/// `objects/evaluation/work_unit/work_unit_store.rs`: `read_work_unit`, `update_work_unit`; [`verdict_of`];
/// `objects/evaluation/work_unit/work_unit_rules/results.rs`: `quotes_found`; [`with_dependencies`], [`joined`].
pub fn apply_assessment(
    db: &UserDatabase,
    evaluation: i64,
    unit: i64,
    assessment: &Assessment,
) -> Result<Vec<String>, AiError> {
    let stored = read_evaluation_plan(db, evaluation)?;
    let this = read_work_unit(db, unit)?;

    ensure!(
        stored.work_units.contains(&unit),
        AiError::Refused("the work unit is not one of the evaluation".to_owned())
    );

    let Some(own) = verdict_of(&assessment.verdict, &stored.possible_verdicts) else {
        return Err(AiError::Refused(format!("the verdict \"{}\" is none of the plan", assessment.verdict)));
    };
    let quotes = quotes_found(db, &this, &assessment.quotes)?;
    let mut left_out = Vec::with_capacity(quotes.missing.len());

    for (name, text) in &quotes.missing {
        left_out.push(format!("The AI quoted a text not found in {name}: \"{text}\""));
    }

    let (verdict, dependency_lines) = with_dependencies(db, &this, own)?;
    let assessed = WorkUnit {
        findings: joined(&assessment.findings, &dependency_lines),
        analysis: assessment.analysis.clone(),
        vulnerability_notes: assessment.vulnerability_notes.clone(),
        notes: joined(&assessment.notes, &left_out),
        quotes: quotes.found,
        verdict: Some(verdict),
        author: Some(Author::Ai),
        ..this
    };

    update_work_unit(db, &assessed)?;

    Ok(left_out)
}

#[cfg(test)]
mod tests {
    //! Tests of the answer: read from its JSON, the quotes checked in the
    //! documents, the verdict made the worst with the dependencies.

    use super::*;

    use crate::objects::evaluation::work_unit::work_unit_store::read_work_unit;
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

        assert!(matches!(read_assessment("It passes."), Err(AiError::Refused(_))));
        assert!(matches!(
            apply_assessment(&db, id, first, &read_assessment(&answer("inconclusive", "")).unwrap()),
            Err(AiError::Refused(_))
        ));
        assert_eq!(read_work_unit(&db, first).unwrap().verdict, None);
    }
}
