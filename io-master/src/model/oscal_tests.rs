//! Unit tests of `oscal.rs`, on the example documents of NIST
//! (`oscal/examples/`).

use super::*;

/// The examples of NIST, one per model the program reads.
const EXAMPLES: [(&str, OscalModel); 4] = [
    (include_str!("../../oscal/examples/basic-catalog.json"), OscalModel::Catalog),
    (include_str!("../../oscal/examples/basic-profile.json"), OscalModel::Profile),
    (include_str!("../../oscal/examples/ifa_assessment-plan-example.json"), OscalModel::AssessmentPlan),
    (include_str!("../../oscal/examples/ifa_assessment-results-example.json"), OscalModel::AssessmentResults),
];

#[test]
fn the_examples_of_nist_are_read_and_written_unchanged() {
    for (text, model) in EXAMPLES {
        let document = OscalDocument::from_json(text).unwrap_or_else(|error| panic!("{}: {error}", model.name()));
        assert_eq!(document.model(), model);

        // What the program writes is the document it read: no field is
        // lost, none added (the order of the keys aside).
        let mut original: Value = serde_json::from_str(text).unwrap();
        original.as_object_mut().unwrap().remove("$schema");
        assert_eq!(document.to_json().unwrap(), original, "{}", model.name());
    }
}

#[test]
fn a_catalog_gives_its_controls_with_their_groups() {
    let OscalDocument::Catalog(catalog) = OscalDocument::from_json(EXAMPLES[0].0).unwrap() else {
        panic!("a catalog");
    };
    let controls = catalog.all_controls().unwrap();

    assert!(!controls.is_empty());
    assert!(controls.iter().all(|(path, _)| !path.is_empty()), "the example has its controls in groups");
}

#[test]
fn a_document_that_breaks_the_schema_is_refused() {
    // A catalog without its required uuid.
    let broken = r#"{"catalog": {"metadata": {"title": "T", "last-modified": "2026-10-18T00:00:00Z",
        "version": "1", "oscal-version": "1.2.3"}}}"#;
    assert!(matches!(OscalDocument::from_json(broken), Err(NotOscal::Schema(_))));

    assert!(matches!(OscalDocument::from_json("not json"), Err(NotOscal::NotJson(_))));
    assert!(matches!(OscalDocument::from_json(r#"{"system-security-plan": {}}"#), Err(NotOscal::UnknownModel(_))));
}

#[test]
fn the_summary_names_the_document() {
    let document = OscalDocument::from_json(EXAMPLES[0].0).unwrap();
    let summary = document.summary();

    assert_eq!(summary.model, OscalModel::Catalog);
    assert_eq!(summary.uuid, document.uuid());
    assert!(!summary.title.is_empty());

    for model in OscalModel::ALL {
        assert_eq!(OscalModel::from_name(model.name()), Some(model));
    }
}
