//! Unit tests of `model.rs`.

use super::*;

#[test]
fn names_of_the_enums_go_there_and_back() {
    for field in ResultField::ALL {
        assert_eq!(ResultField::from_name(field.name()), Some(field));
        assert!(!field.title().is_empty() && !field.description().is_empty());
    }

    for origin in [ResultOrigin::Evaluator, ResultOrigin::Ai, ResultOrigin::AiCorrected] {
        assert_eq!(ResultOrigin::from_name(origin.name()), Some(origin));
    }

    for action in [
        JournalAction::Imported,
        JournalAction::Proposed,
        JournalAction::Accepted,
        JournalAction::Corrected,
        JournalAction::Rejected,
        JournalAction::Written,
    ] {
        assert_eq!(JournalAction::from_name(action.name()), Some(action));
    }

    assert_eq!(ResultField::from_name("verdict"), None);
}

#[test]
fn the_four_results_are_those_the_user_named() {
    let titles: Vec<&str> = ResultField::ALL.iter().map(|field| field.title()).collect();

    assert_eq!(titles, ["Findings", "Analysis", "Assessment and verdict proposition", "Vulnerabilities and notes"]);
}

#[test]
fn an_evaluation_needs_a_name_and_a_product() {
    let fields = EvaluationFields {
        name: "Meter X 2.1".to_owned(),
        product: "Meter X".to_owned(),
        product_version: String::new(),
        manufacturer: String::new(),
        evaluator: String::new(),
    };
    assert!(fields.check().is_ok());

    let nameless = EvaluationFields { name: "  ".to_owned(), ..fields.clone() };
    assert!(matches!(nameless.check(), Err(EvaluationError::Invalid("name", _))));

    let without_product = EvaluationFields { product: String::new(), ..fields.clone() };
    assert!(matches!(without_product.check(), Err(EvaluationError::Invalid("product", _))));

    let broken = EvaluationFields { manufacturer: "ACME\u{0}".to_owned(), ..fields.clone() };
    assert!(matches!(broken.check(), Err(EvaluationError::Invalid("manufacturer", _))));

    let long = EvaluationFields { name: "x".repeat(MAX_NAME + 1), ..fields };
    assert!(matches!(long.check(), Err(EvaluationError::Invalid("name", _))));
}

#[test]
fn a_result_may_hold_lines_but_no_other_control_characters() {
    assert!(check_text("text", "line 1\nline 2\tend", MAX_RESULT, true).is_ok());
    assert!(check_text("text", "bell\u{7}", MAX_RESULT, true).is_err());
    assert!(check_text("name", "two\nlines", MAX_NAME, false).is_err());
}
