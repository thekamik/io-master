//! The results of a work unit as the evaluator writes them (user,
//! 2026-10-30): the evaluator decides (P1), so a result written or changed
//! by the evaluator is of author User, and the AI never writes it again.
//!
//! **A quote stands word for word in its source** (user, 2026-10-30): a
//! quote is a fragment of a document of the product or, in a functional
//! test, of a proof of the laboratory (user, 2026-11-02), and whether it
//! stands there is checked here, for the evaluator and for the AI alike
//! (`objects/evaluation/work_unit/work_unit_ai/assessment_answer.rs` writes the notes of
//! the quotes of the AI not found); the spaces and line ends are passed over.
//!
//! ```text
//! save_results ──► the evaluation in progress, the texts within their limits, the verdict one of the plan,
//!                  each quote found in its document or proof ──► update_work_unit, author User
//! quotes_found ──► the quotes that stand in the documents or proofs of the work unit, and those that do not
//! ```

use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::error::DomainError;
use crate::objects::evaluation::evaluation_plan::evaluation_plan_rules::evaluation::evaluation_in_progress;
use crate::objects::evaluation::lab_proof::lab_proof_store::read_lab_proof;
use crate::objects::evaluation::product_document::product_document_store::{read_document_text, read_product_document};
use crate::objects::evaluation::work_unit::work_unit_model::{Author, ProofQuote, Quote, Verdict, WorkUnit};
use crate::objects::evaluation::work_unit::work_unit_store::{MAX_QUOTES, read_work_unit, update_work_unit};
use crate::objects::text::check_length;

/// The longest text of a result, in characters.
const MAX_RESULT: usize = 100_000;

/// The quotes proposed, parted by whether they stand in their sources.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuotesFound {
    /// Those found in a document, as quotes of their documents.
    pub found: Vec<Quote>,
    /// Those found in a proof, as quotes of their proofs.
    pub proofs_found: Vec<ProofQuote>,
    /// Those not found: the name of a document or a proof and the text, as
    /// given.
    pub missing: Vec<(String, String)>,
}

/// A source of quotes of a work unit: its id, its name and its text, the
/// spaces made one.
type QuoteSource = (i64, String, String);

/// `text` with every run of spaces, tabs and line ends made one space, so
/// that a quote is found however the lines of its document were broken.
///
/// Called by: [`quotes_found`].
fn spaced(text: &str) -> String {
    text.split_whitespace().collect::<Vec<&str>>().join(" ")
}

/// The texts the quotes of the work unit `unit` may come from: its documents
/// and its proofs, each with its name, the spaces made one.
///
/// Input: `unit`, a work unit.
/// Output: the documents and the proofs (`([(3, "ST", "The TOE is ...")],
/// [(6, "Scan", "port 22 closed")])`); a proof not given yet has an empty
/// text; an error of the database.
///
/// Called by: [`quotes_found`].
/// Calls: `objects/evaluation/product_document/product_document_store.rs`: `read_product_document`,
/// `read_document_text`; `objects/evaluation/lab_proof/lab_proof_store.rs`: `read_lab_proof`; [`spaced`].
fn quote_sources(db: &UserDatabase, unit: &WorkUnit) -> Result<(Vec<QuoteSource>, Vec<QuoteSource>), DomainError> {
    let proofs = unit.input_functional.as_deref().unwrap_or_default();
    let mut documents = Vec::with_capacity(unit.input_product_documents.len());
    let mut texts = Vec::with_capacity(proofs.len());

    for id in &unit.input_product_documents {
        documents.push((*id, read_product_document(db, *id)?.name, spaced(&read_document_text(db, *id)?)));
    }

    for id in proofs {
        let proof = read_lab_proof(db, *id)?;

        texts.push((*id, proof.name, spaced(proof.proof.as_deref().unwrap_or_default())));
    }

    Ok((documents, texts))
}

/// The source among `sources` named `name` (without regard to case, and
/// without a version the model may add in parentheses: "ST (version 1.0)" is
/// "ST") whose text holds `wanted`.
///
/// Input: `sources`, from [`quote_sources`]; `name`, the name the quote gives;
/// `wanted`, the quote, the spaces made one.
/// Output: the id of the source (`Some(3)`); `None` when none holds it.
///
/// Called by: [`quotes_found`].
fn place_of(sources: &[QuoteSource], name: &str, wanted: &str) -> Option<i64> {
    let bare = match name.split_once(" (") {
        Some((head, _)) => head.trim(),
        None => name.trim(),
    };

    for (id, source, text) in sources {
        let source = source.trim();

        if (source.eq_ignore_ascii_case(name.trim()) || source.eq_ignore_ascii_case(bare)) && text.contains(wanted) {
            return Some(*id);
        }
    }

    None
}

/// The quotes `proposed` that stand word for word in the documents or the
/// proofs of the work unit `unit`, and those that do not.
///
/// Input:
/// - `unit`: a work unit; its sources are its `input_product_documents` and
///   its `input_functional`.
/// - `proposed`: the name of a document or a proof and the text quoted from
///   it (`[("ST", "The TOE is the router R7")]`); the name is matched without
///   regard to case, a document first.
///
/// Output: the quotes found and those not found (`QuotesFound { found:
/// [Quote { product_document: 3, .. }], proofs_found: [], missing: [("IXIT",
/// "...")] }`). An error when the database fails.
///
/// Called by: [`check_quote`]; `objects/evaluation/work_unit/work_unit_ai/assessment_answer.rs`:
/// `apply_assessment`.
/// Calls: [`quote_sources`], [`spaced`], [`place_of`].
pub fn quotes_found(
    db: &UserDatabase,
    unit: &WorkUnit,
    proposed: &[(String, String)],
) -> Result<QuotesFound, DomainError> {
    let (documents, proofs) = quote_sources(db, unit)?;
    let mut quotes =
        QuotesFound { found: Vec::with_capacity(proposed.len()), proofs_found: Vec::new(), missing: Vec::new() };

    for (name, text) in proposed {
        let wanted = spaced(text);

        if let Some(product_document) = place_of(&documents, name, &wanted) {
            quotes.found.push(Quote { product_document, text: text.clone() });
        } else if let Some(lab_proof) = place_of(&proofs, name, &wanted) {
            quotes.proofs_found.push(ProofQuote { lab_proof, text: text.clone() });
        } else {
            quotes.missing.push((name.clone(), text.clone()));
        }
    }

    Ok(quotes)
}

/// Whether the quote `text` of the document `document` stands word for word
/// in it, for the work unit `unit`.
///
/// Input: `unit`, a work unit; `document`, the name of one of its documents;
/// `text`, the text quoted.
/// Output: `true` when it is found; an error of the database.
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
    /// The quotes of the proofs of a functional test.
    pub proof_quotes: Vec<ProofQuote>,
    pub verdict: Option<Verdict>,
}

/// Refuses a quote of the work unit `unit` that is not of one of its
/// documents, or that is not found in it word for word.
///
/// Input: `unit`, the work unit as stored; `quotes`, the quotes the
/// evaluator gives.
/// Output: nothing; `DomainError::Refused` for more than `MAX_QUOTES`, a
/// quote of another document or one not found, an error of the database.
///
/// Called by: [`save_results`].
/// Calls: `objects/evaluation/product_document/product_document_store.rs`: `read_product_document`;
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

/// Refuses a quote of a proof of the work unit `unit` that is not of one of
/// its proofs, or that is not found in it word for word.
///
/// Input: `unit`, the work unit as stored; `quotes`, the quotes of proofs the
/// evaluator gives.
/// Output: nothing; `DomainError::Refused` for more than `MAX_QUOTES`, a
/// quote of another proof or one not found, an error of the database.
///
/// Called by: [`save_results`].
/// Calls: `objects/evaluation/lab_proof/lab_proof_store.rs`: `read_lab_proof`; [`quotes_found`].
fn check_proof_quotes(db: &UserDatabase, unit: &WorkUnit, quotes: &[ProofQuote]) -> Result<(), DomainError> {
    ensure!(quotes.len() <= MAX_QUOTES, DomainError::Refused(format!("more than {MAX_QUOTES} quotes of proofs")));

    for quote in quotes {
        ensure!(
            unit.input_functional.as_deref().unwrap_or_default().contains(&quote.lab_proof),
            DomainError::Refused("a quote of a proof the work unit does not need".to_owned())
        );

        let name = read_lab_proof(db, quote.lab_proof)?.name;
        let found = quotes_found(db, unit, &[(name.clone(), quote.text.clone())])?;

        ensure!(
            found.proofs_found.contains(quote),
            DomainError::Refused(format!("the quote is not found in {name}: \"{}\"", quote.text))
        );
    }

    Ok(())
}

/// Writes the results `results` of the work unit `unit` of the evaluation
/// `evaluation`, as the evaluator gives them: of author User.
///
/// Input: `evaluation`, the id of an evaluation in progress; `unit`, the id
/// of one of its work units; `results`, as the evaluator gives them.
/// Output: nothing; `DomainError::Refused` unless the evaluation is in
/// progress, the work unit is one of it, the verdict is one of the plan,
/// every text is within [`MAX_RESULT`] and each quote stands in its document
/// or proof.
///
/// Called by: `panels/views/evaluation/work_units.rs`: `evaluation_unit_save`.
/// Calls: `objects/evaluation/evaluation_plan/evaluation_plan_rules/evaluation.rs`: `evaluation_in_progress`;
/// `objects/evaluation/work_unit/work_unit_store.rs`: `read_work_unit`, `update_work_unit`; `objects/text.rs`:
/// `check_length`; [`check_quotes`], [`check_proof_quotes`].
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
    check_proof_quotes(db, &this, &results.proof_quotes)?;

    let written = WorkUnit {
        findings: results.findings.trim().to_owned(),
        analysis: results.analysis.trim().to_owned(),
        vulnerability_notes: results.vulnerability_notes.trim().to_owned(),
        notes: results.notes.trim().to_owned(),
        quotes: results.quotes.clone(),
        proof_quotes: results.proof_quotes.clone(),
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

    use crate::objects::evaluation::evaluation_plan::evaluation_plan_store::read_evaluation_plan;
    use crate::objects::evaluation::lab_proof::lab_proof_model::LabProof;
    use crate::objects::evaluation::lab_proof::lab_proof_rules::give_proof;
    use crate::objects::evaluation::work_unit::work_unit_store::read_work_units;
    use crate::test_support::database::database;
    use crate::test_support::evaluation::{evaluation, evaluation_needing_inputs};

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
            proof_quotes: Vec::new(),
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
            ("ST (version 1.0)".to_owned(), "Its TSF filters packets.".to_owned()),
        ];
        let quotes = quotes_found(&db, &unit, &proposed).unwrap();

        assert_eq!(quotes.found.len(), 2);
        assert_eq!(quotes.missing, [proposed[1].clone()]);
    }

    #[test]
    fn a_quote_of_a_proof_is_found_in_its_text_and_checked_for_the_evaluator() {
        let (_directory, db) = database();
        let id = evaluation_needing_inputs(&db);
        let unit = read_work_units(&db, id).unwrap().remove(0);
        let scan = unit.input_functional.clone().unwrap()[0];
        let given = LabProof {
            id: scan,
            name: "Scan".to_owned(),
            proof: Some("Port 22 is closed.\nPort 443 answers with TLS 1.3.".to_owned()),
            performed: Some(1_793_000_000),
            tester: Some("K. K.".to_owned()),
            equipment: Some("nmap 7.95".to_owned()),
        };

        give_proof(&db, id, &given).unwrap();

        let proposed = [("scan".to_owned(), "Port 443 answers with TLS 1.3.".to_owned())];
        let quotes = quotes_found(&db, &unit, &proposed).unwrap();

        assert_eq!(quotes.proofs_found, [ProofQuote { lab_proof: scan, text: proposed[0].1.clone() }]);
        assert!(quotes.found.is_empty() && quotes.missing.is_empty());

        let results = Results {
            findings: String::new(),
            analysis: "The scan shows TLS 1.3.".to_owned(),
            vulnerability_notes: String::new(),
            notes: String::new(),
            quotes: Vec::new(),
            proof_quotes: quotes.proofs_found,
            verdict: Some(Verdict::Pass),
        };

        save_results(&db, id, unit.id, &results).unwrap();
        assert_eq!(read_work_unit(&db, unit.id).unwrap().proof_quotes.len(), 1);

        let invented = ProofQuote { lab_proof: scan, text: "Port 23 is open.".to_owned() };

        assert!(matches!(
            save_results(&db, id, unit.id, &Results { proof_quotes: vec![invented], ..results }),
            Err(DomainError::Refused(_))
        ));
    }
}
