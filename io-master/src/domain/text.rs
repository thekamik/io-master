//! The one rule of a text the user gives (user, 2026-11-01): trimmed, and
//! within its limit; a name or a record that must be given is not empty.
//! Every model of `domain/models/` checks its texts here, so that a text is
//! refused in the same words wherever it is given.
//!
//! ```text
//! a name, a field of a record ──► checked_text ──► trimmed, not empty, at most `limit` characters
//! a result, a note            ──► check_length ──► at most `limit` characters (it may be empty)
//! ```

use crate::domain::error::DomainError;

/// The longest name given by the user (a plan, a place of a document or of
/// a proof), and a field of the identification of an evaluation (a product,
/// a client, an evaluator), in characters.
pub const MAX_NAME: usize = 200;

/// The text `text` trimmed; refused when it is empty or over `limit`
/// characters.
///
/// Input:
/// - `text`: the text as the user gave it (" Router R7 ").
/// - `what`: what it is, for the reason of a refusal ("the product").
/// - `limit`: its most characters, at least 1.
///
/// Output: the text trimmed ("Router R7"); `DomainError::Refused` ("the
/// product is missing", "the product over 200 characters").
///
/// Called by: `models/evaluation/evaluation_plan/planning.rs`: `create_plan`,
/// `add_document_place`; `models/evaluation/evaluation_plan/evaluation.rs`:
/// `new_evaluation`; `models/evaluation/work_unit/parameters.rs`:
/// `add_proof_place`; `models/evaluation/product_document.rs`:
/// `give_document`; `models/evaluation/lab_proof.rs`: `give_proof`.
pub fn checked_text(text: &str, what: &str, limit: usize) -> Result<String, DomainError> {
    let text = text.trim();

    ensure!(!text.is_empty(), DomainError::Refused(format!("{what} is missing")));
    ensure!(text.chars().count() <= limit, DomainError::Refused(format!("{what} over {limit} characters")));

    Ok(text.to_owned())
}

/// Refuses the text `text` over `limit` characters; an empty text is
/// allowed.
///
/// Input: `text`, as the user gave it; `what`, for the reason ("the
/// findings"); `limit`, its most characters.
///
/// Output: nothing; `DomainError::Refused` ("the findings over 100000
/// characters").
///
/// Called by: `panels/logic/evaluation/results.rs`: `save_results`.
pub fn check_length(text: &str, what: &str, limit: usize) -> Result<(), DomainError> {
    ensure!(text.chars().count() <= limit, DomainError::Refused(format!("{what} over {limit} characters")));

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Tests of the rule of a text.

    use super::*;

    #[test]
    fn a_text_is_trimmed_and_refused_when_empty_or_too_long() {
        assert_eq!(checked_text(" Router R7 ", "the product", 9).unwrap(), "Router R7");
        assert!(
            matches!(checked_text("  ", "the product", 9), Err(DomainError::Refused(reason)) if reason == "the product is missing")
        );
        assert!(matches!(checked_text("Router R7X", "the product", 9), Err(DomainError::Refused(_))));
        assert!(check_length("", "the notes", 3).is_ok());
        assert!(check_length("four", "the notes", 3).is_err());
    }
}
