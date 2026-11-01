//! The results of a work unit as the evaluator writes them (user,
//! 2026-10-30): the evaluator decides (P1), so a result written or changed
//! by the evaluator is of author User, and the AI never writes it again.
//!
//! **A quote stands word for word in its document** (user, 2026-10-30): a
//! quote is only a fragment of a document of the product, and whether it
//! stands there is checked here, for the evaluator and for the AI alike
//! (`ai/models/evaluation/work_unit/assessment_answer.rs` writes the notes of
//! the quotes of the AI not found); the spaces and line ends are passed over.
//!
//! ```text
//! save_results ──► the evaluation in progress, the texts within their limits, the verdict one of the plan,
//!                  each quote found in its document (check_quote) ──► update_work_unit, author User
//! quotes_found ──► the quotes that stand in the documents of the work unit, and those that do not
//! ```

use crate::db::models::evaluation::product_document::{read_document_text, read_product_document};
use crate::db::models::evaluation::work_unit::{MAX_QUOTES, read_work_unit, update_work_unit};
use crate::domain::error::DomainError;
use crate::domain::models::evaluation::evaluation_plan::evaluation::evaluation_in_progress;
use crate::domain::text::check_length;
use crate::infrastructure::storage::user_database::UserDatabase;
use crate::model::evaluation::work_unit::{Author, Quote, Verdict, WorkUnit};

/// The longest text of a result, in characters.
const MAX_RESULT: usize = 100_000;

/// The quotes proposed, parted by whether they stand in their documents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuotesFound {
    /// Those found, as quotes of their documents.
    pub found: Vec<Quote>,
    /// Those not found: the name of a document and the text, as given.
    pub missing: Vec<(String, String)>,
}

/// `text` with every run of spaces, tabs and line ends made one space, so
/// that a quote is found however the lines of its document were broken.
///
/// Called by: [`quotes_found`].
fn spaced(text: &str) -> String {
    text.split_whitespace().collect::<Vec<&str>>().join(" ")
}

/// The quotes `proposed` that stand word for word in the documents of the
/// work unit `unit`, and those that do not.
///
/// Input:
/// - `unit`: a work unit; its documents are those of `input_product_documents`.
/// - `proposed`: the name of a document and the text quoted from it
///   (`[("ST", "The TOE is the router R7")]`); the name is matched without
///   regard to case.
///
/// Output: the quotes found and those not found (`QuotesFound { found:
/// [Quote { product_document: 3, .. }], missing: [("IXIT", "...")] }`). An
/// error when the database fails.
///
/// Called by: [`check_quote`]; `ai/models/evaluation/work_unit/assessment_answer.rs`:
/// `apply_assessment`.
/// Calls: `db/models/evaluation/product_document.rs`: `read_product_document`,
/// `read_document_text`; [`spaced`].
pub fn quotes_found(
    db: &UserDatabase,
    unit: &WorkUnit,
    proposed: &[(String, String)],
) -> Result<QuotesFound, DomainError> {
    let mut texts = Vec::with_capacity(unit.input_product_documents.len());

    for id in &unit.input_product_documents {
        texts.push((*id, read_product_document(db, *id)?.name, spaced(&read_document_text(db, *id)?)));
    }

    let mut quotes = QuotesFound { found: Vec::with_capacity(proposed.len()), missing: Vec::new() };

    for (name, text) in proposed {
        let wanted = spaced(text);
        let mut found = None;

        for (id, document, content) in &texts {
            if document.trim().eq_ignore_ascii_case(name) && content.contains(&wanted) {
                found = Some(*id);
            }
        }

        match found {
            Some(product_document) => quotes.found.push(Quote { product_document, text: text.clone() }),
            None => quotes.missing.push((name.clone(), text.clone())),
        }
    }

    Ok(quotes)
}

/// Whether the quote `text` of the document `document` stands word for word
/// in it, for the work unit `unit`.
///
/// Called by: [`check_quotes`].
/// Calls: [`quotes_found`].
fn check_quote(db: &UserDatabase, unit: &WorkUnit, document: &str, text: &str) -> Result<bool, DomainError> {
    let quotes = quotes_found(db, unit, &[(document.to_owned(), text.to_owned())])?;

    Ok(!quotes.found.is_empty())
}

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

/// Refuses a quote of the work unit `unit` that is not of one of its
/// documents, or that is not found in it word for word.
///
/// Called by: [`save_results`].
/// Calls: `db/models/evaluation/product_document.rs`: `read_product_document`;
/// [`check_quote`].
fn check_quotes(db: &UserDatabase, unit: &WorkUnit, quotes: &[Quote]) -> Result<(), DomainError> {
    ensure!(quotes.len() <= MAX_QUOTES, DomainError::Refused(format!("more than {MAX_QUOTES} quotes")));

    for quote in quotes {
        ensure!(
            unit.input_product_documents.contains(&quote.product_document),
            DomainError::Refused("a quote of a document the work unit does not need".to_owned())
        );

        let name = read_product_document(db, quote.product_document)?.name;

        ensure!(
            check_quote(db, unit, &name, &quote.text)?,
            DomainError::Refused(format!("the quote is not found in {name}: \"{}\"", quote.text))
        );
    }

    Ok(())
}

/// Writes the results `results` of the work unit `unit` of the evaluation
/// `evaluation`, as the evaluator gives them: of author User. Refused unless
/// the evaluation is in progress, the verdict is one of the plan and each
/// quote stands in its document.
///
/// Called by: `panels/views/evaluation/work_units.rs`: `evaluation_unit_save`.
/// Calls: `domain/models/evaluation/evaluation_plan/evaluation.rs`: `evaluation_in_progress`; `read_work_unit`, `domain/text.rs`: `check_length`;
/// [`check_quotes`], `update_work_unit`.
pub fn save_results(db: &UserDatabase, evaluation: i64, unit: i64, results: &Results) -> Result<(), DomainError> {
    let stored = evaluation_in_progress(db, evaluation)?;
    let this = read_work_unit(db, unit)?;

    ensure!(
        stored.work_units.contains(&unit),
        DomainError::Refused("the work unit is not one of the evaluation".to_owned())
    );
    ensure!(
        results.verdict.is_none_or(|verdict| stored.possible_verdicts.contains(&verdict)),
        DomainError::Refused("the verdict is none of the plan".to_owned())
    );
    check_length(&results.findings, "the findings", MAX_RESULT)?;
    check_length(&results.analysis, "the analysis", MAX_RESULT)?;
    check_length(&results.vulnerability_notes, "the vulnerability notes", MAX_RESULT)?;
    check_length(&results.notes, "the notes", MAX_RESULT)?;
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
    //! quote not found and a verdict of no plan refused; a quote found.

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
            Err(DomainError::Refused(_))
        ));
        assert!(matches!(
            save_results(&db, id, first, &Results { verdict: Some(Verdict::Inconclusive), ..results }),
            Err(DomainError::Refused(_))
        ));
    }

    #[test]
    fn a_quote_is_found_however_its_lines_were_broken_and_its_document_named() {
        let (_directory, db) = database();
        let (_, first, _) = evaluation(&db);
        let unit = read_work_unit(&db, first).unwrap();
        let proposed = [
            ("st".to_owned(), "Its TSF filters\npackets.".to_owned()),
            ("IXIT".to_owned(), "Its TSF filters packets.".to_owned()),
        ];
        let quotes = quotes_found(&db, &unit, &proposed).unwrap();

        assert_eq!(quotes.found.len(), 1);
        assert_eq!(quotes.missing, [proposed[1].clone()]);
    }
}
