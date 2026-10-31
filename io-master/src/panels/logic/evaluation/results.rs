//! The results of a work unit as the evaluator writes them (user,
//! 2026-10-30): the evaluator decides (P1), so a result written or changed
//! by the evaluator is of author User, and the AI never writes it again. Its
//! quotes are checked in their documents as those of the AI are.
//!
//! ```text
//! save_results ──► the evaluation in progress, the texts within their limits, the verdict one of the plan,
//!                  each quote found in its document ──► update_work_unit, author User
//! ```

use crate::db::models::evaluation::product_document::read_product_document;
use crate::db::models::evaluation::work_unit::{MAX_QUOTES, read_work_unit, update_work_unit};
use crate::db::storage::user_database::UserDatabase;
use crate::model::evaluation::work_unit::{Author, Quote, Verdict, WorkUnit};
use crate::panels::logic::evaluation::answer::check_quote;
use crate::panels::logic::evaluation::error::EvaluationError;
use crate::panels::logic::evaluation::evaluations::evaluation_in_progress;

/// The longest text of a result, in characters.
const MAX_RESULT: usize = 100_000;

/// The results of a work unit as the evaluator gives them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Results {
    pub findings: String,
    pub analysis: String,
    pub vulnerability_notes: String,
    pub notes: String,
    pub quotes: Vec<Quote>,
    pub verdict: Option<Verdict>,
}

/// Refuses a text of a result over [`MAX_RESULT`] characters.
///
/// Called by: [`save_results`].
fn check_length(text: &str, what: &str) -> Result<(), EvaluationError> {
    ensure!(
        text.chars().count() <= MAX_RESULT,
        EvaluationError::Refused(format!("{what} over {MAX_RESULT} characters"))
    );

    Ok(())
}

/// Refuses a quote of the work unit `unit` that is not of one of its
/// documents, or that is not found in it word for word.
///
/// Called by: [`save_results`].
/// Calls: `read_product_document`, `check_quote`.
fn check_quotes(db: &UserDatabase, unit: &WorkUnit, quotes: &[Quote]) -> Result<(), EvaluationError> {
    ensure!(quotes.len() <= MAX_QUOTES, EvaluationError::Refused(format!("more than {MAX_QUOTES} quotes")));

    for quote in quotes {
        ensure!(
            unit.input_product_documents.contains(&quote.product_document),
            EvaluationError::Refused("a quote of a document the work unit does not need".to_owned())
        );

        let name = read_product_document(db, quote.product_document)?.name;

        ensure!(
            check_quote(db, unit, &name, &quote.text)?,
            EvaluationError::Refused(format!("the quote is not found in {name}: \"{}\"", quote.text))
        );
    }

    Ok(())
}

/// Writes the results `results` of the work unit `unit` of the evaluation
/// `evaluation`, as the evaluator gives them: of author User. Refused unless
/// the evaluation is in progress, the verdict is one of the plan and each
/// quote stands in its document.
///
/// Called by: `panels/views/evaluation/work_units.rs` (`evaluation_unit_save`).
/// Calls: `evaluation_in_progress`, `read_work_unit`, [`check_length`],
/// [`check_quotes`], `update_work_unit`.
pub fn save_results(db: &UserDatabase, evaluation: i64, unit: i64, results: &Results) -> Result<(), EvaluationError> {
    let stored = evaluation_in_progress(db, evaluation)?;
    let this = read_work_unit(db, unit)?;

    ensure!(
        stored.work_units.contains(&unit),
        EvaluationError::Refused("the work unit is not one of the evaluation".to_owned())
    );
    ensure!(
        results.verdict.is_none_or(|verdict| stored.possible_verdicts.contains(&verdict)),
        EvaluationError::Refused("the verdict is none of the plan".to_owned())
    );
    check_length(&results.findings, "the findings")?;
    check_length(&results.analysis, "the analysis")?;
    check_length(&results.vulnerability_notes, "the vulnerability notes")?;
    check_length(&results.notes, "the notes")?;
    check_quotes(db, &this, &results.quotes)?;

    let written = WorkUnit {
        findings: results.findings.trim().to_owned(),
        analysis: results.analysis.trim().to_owned(),
        vulnerability_notes: results.vulnerability_notes.trim().to_owned(),
        notes: results.notes.trim().to_owned(),
        quotes: results.quotes.clone(),
        verdict: results.verdict,
        author: Some(Author::User),
        ..this
    };

    Ok(update_work_unit(db, &written)?)
}

#[cfg(test)]
mod tests {
    //! Tests of the results of the evaluator: written as of author User, a
    //! quote not found and a verdict of no plan refused.

    use super::*;

    use crate::db::models::evaluation::evaluation_plan::read_evaluation_plan;
    use crate::test_support::database::database;
    use crate::test_support::evaluation::evaluation;

    #[test]
    fn the_results_of_the_evaluator_are_of_author_user_and_their_quotes_checked() {
        let (_directory, db) = database();
        let (id, first, _) = evaluation(&db);
        let st = read_evaluation_plan(&db, id).unwrap().product_documents[0];
        let results = Results {
            findings: String::new(),
            analysis: " The ST names the TOE. ".to_owned(),
            vulnerability_notes: String::new(),
            notes: String::new(),
            quotes: vec![Quote { product_document: st, text: "The TOE is the router R7".to_owned() }],
            verdict: Some(Verdict::Pass),
        };

        save_results(&db, id, first, &results).unwrap();

        let stored = read_work_unit(&db, first).unwrap();

        assert_eq!((stored.author, stored.analysis.as_str()), (Some(Author::User), "The ST names the TOE."));

        let invented = Quote { product_document: st, text: "The TOE is a firewall".to_owned() };

        assert!(matches!(
            save_results(&db, id, first, &Results { quotes: vec![invented], ..results.clone() }),
            Err(EvaluationError::Refused(_))
        ));
        assert!(matches!(
            save_results(&db, id, first, &Results { verdict: Some(Verdict::Inconclusive), ..results }),
            Err(EvaluationError::Refused(_))
        ));
    }
}
