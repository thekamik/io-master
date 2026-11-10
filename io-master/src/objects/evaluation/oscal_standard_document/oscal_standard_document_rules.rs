//! The standards (`OscalStandardDocument`): an OSCAL catalog imported by the
//! user, checked against the JSON schema of NIST before it is stored, and the
//! controls of a catalog, from which the work units of a plan are made.
//!
//! ```text
//! the page ──the text of a file──► import_standard ──► check_catalog ──► create_oscal_standard_document
//! a standard of a plan ──► catalog_controls ──► evaluation_plan_rules/work_units.rs (the work units made)
//! a standard ──► oscal_standard_document_fields ──► the values of its markers in a report
//! ```
//!
//! The schema is the file of NIST in `oscal/schema/v1.2.3/`, built into
//! the program; nothing is fetched. The tree of a catalog (groups in
//! groups, controls in controls, parts in parts) is walked with an explicit
//! stack bounded by [`MAX_OBJECTS`], never by recursion (rule 1).

use jsonschema::Validator;
use serde_json::Value;

use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::error::DomainError;
use crate::objects::evaluation::oscal_standard_document::oscal_standard_document_model::OscalStandardDocument;
use crate::objects::evaluation::oscal_standard_document::oscal_standard_document_store::create_oscal_standard_document;

/// The largest document read, in bytes (the CEM is 3 MB).
pub const MAX_DOCUMENT: usize = 32 * 1024 * 1024;

/// The most objects of a catalog walked (groups, controls, parts): a larger
/// catalog is refused.
pub const MAX_OBJECTS: usize = 200_000;

/// How many messages of a failed check are kept for the user.
const MAX_MESSAGES: usize = 10;

/// The schema of an OSCAL catalog, as NIST publishes it.
const CATALOG_SCHEMA: &str = include_str!("../../../../oscal/schema/v1.2.3/oscal_catalog_schema.json");

/// Why a document is no catalog the program takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OscalError {
    /// The document is over [`MAX_DOCUMENT`].
    TooLarge,
    /// The document is no JSON; holds the reason.
    NotJson(String),
    /// The document breaks the schema of an OSCAL catalog; holds the first
    /// messages of the check.
    NotCatalog(Vec<String>),
    /// The catalog holds more than [`MAX_OBJECTS`] objects.
    TooMany,
}

impl std::fmt::Display for OscalError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OscalError::TooLarge => write!(formatter, "the document is over {MAX_DOCUMENT} bytes"),
            OscalError::NotJson(reason) => write!(formatter, "the document is no JSON: {reason}"),
            OscalError::NotCatalog(messages) => {
                write!(formatter, "the document is no OSCAL catalog: {}", messages.join("; "))
            }
            OscalError::TooMany => write!(formatter, "the catalog holds more than {MAX_OBJECTS} objects"),
        }
    }
}

impl std::error::Error for OscalError {}

/// A catalog that passed the check: its title and version, as its metadata
/// gives them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedCatalog {
    pub title: String,
    pub version: String,
}

/// A control of a catalog, as a work unit is made from it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogControl {
    /// The `id` of the control in the JSON ("APE_INT.1-1", "s1.1.1").
    pub oscal_key: String,
    /// Its label in the standard; its id when it has none.
    pub label: String,
    /// The text of its part `statement` (with the parts inside it); its
    /// title when it has none.
    pub requirement: String,
    /// The text of its part `guidance` (with the parts inside it). A
    /// control without one gives its other parts beside the statement
    /// here, each under its name: a standard may name its guidance its own
    /// way ("test-units", "assessment-method"). Empty when it has neither.
    pub guidance: String,
}

/// The validator of the schema of NIST, compiled from the text built into
/// the program.
///
/// Output: the validator; `OscalError::NotCatalog` when the schema built in
/// cannot be read, which a build of the program never gives.
///
/// Called by: [`check_catalog`].
fn validator() -> Result<Validator, OscalError> {
    let schema: Value = serde_json::from_str(CATALOG_SCHEMA)
        .map_err(|error| OscalError::NotCatalog(vec![format!("the schema of NIST: {error}")]))?;

    jsonschema::validator_for(&schema)
        .map_err(|error| OscalError::NotCatalog(vec![format!("the schema of NIST: {error}")]))
}

/// The text of a field of the metadata of a catalog.
///
/// Input: `document`, a catalog; `field`, a field of its metadata ("title").
/// Output: its text ("Common Methodology for Information Technology Security
/// Evaluation"); empty when the catalog has no such field.
///
/// Called by: [`check_catalog`].
fn metadata_text(document: &Value, field: &str) -> String {
    document.pointer(&format!("/catalog/metadata/{field}")).and_then(Value::as_str).unwrap_or_default().to_owned()
}

/// Checks that `text` is an OSCAL catalog: JSON within its limit, valid
/// against the schema of NIST.
///
/// Input: `text`, the document as imported.
/// Output: the document read and its title and version (`(document,
/// CheckedCatalog { title: "CEM", version: "3.1 R5" })`); `OscalError::TooLarge`
/// over [`MAX_DOCUMENT`], `OscalError::NotJson`, `OscalError::NotCatalog`
/// with at most [`MAX_MESSAGES`] messages of the check.
///
/// Called by: [`import_standard`].
/// Calls: [`validator`], [`metadata_text`].
pub fn check_catalog(text: &str) -> Result<(Value, CheckedCatalog), OscalError> {
    ensure!(text.len() <= MAX_DOCUMENT, OscalError::TooLarge);

    let document: Value = serde_json::from_str(text).map_err(|error| OscalError::NotJson(error.to_string()))?;

    ensure!(
        document.get("catalog").is_some(),
        OscalError::NotCatalog(vec!["the root holds no \"catalog\"".to_owned()])
    );

    let validator = validator()?;
    let mut messages = Vec::with_capacity(MAX_MESSAGES);

    for error in validator.iter_errors(&document).take(MAX_MESSAGES) {
        messages.push(format!("{}: {error}", error.instance_path()));
    }

    ensure!(messages.is_empty(), OscalError::NotCatalog(messages));

    let checked =
        CheckedCatalog { title: metadata_text(&document, "title"), version: metadata_text(&document, "version") };

    Ok((document, checked))
}

/// What an object of the tree of a catalog is: OSCAL keeps groups only in
/// the arrays `groups` and controls only in the arrays `controls`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Group,
    Control,
}

/// Puts the objects of the array `key` of `object` on `stack` as `kind`,
/// the last first, so that the stack gives them in the order of the
/// document.
///
/// Input: `stack`, the objects still to walk; `object`, a group or a control;
/// `key`, `"groups"` or `"controls"`; `kind`, what the objects of that array
/// are.
///
/// Called by: [`catalog_controls`].
fn push_items<'a>(stack: &mut Vec<(Kind, &'a Value)>, object: &'a Value, key: &str, kind: Kind) {
    if let Some(items) = object.get(key).and_then(Value::as_array) {
        for item in items.iter().rev() {
            stack.push((kind, item));
        }
    }
}

/// The text of a part and of the parts inside it, each on a line of its
/// own, a part with a label ("a.") led by it.
///
/// Input: `part`, a part of a control.
/// Output: its text ("The developer shall ...\na. describe ..."); an
/// `OscalError::TooMany` for more than [`MAX_OBJECTS`] parts.
///
/// Called by: [`parts_named`].
/// Calls: [`label_of`].
fn part_text(part: &Value) -> Result<String, OscalError> {
    let mut stack = vec![part];
    let mut lines = Vec::new();

    for _ in 0..MAX_OBJECTS {
        let Some(current) = stack.pop() else {
            return Ok(lines.join("\n"));
        };

        if let Some(prose) = current.get("prose").and_then(Value::as_str) {
            match label_of(current) {
                Some(label) => lines.push(format!("{label} {prose}")),
                None => lines.push(prose.to_owned()),
            }
        }

        if let Some(parts) = current.get("parts").and_then(Value::as_array) {
            for inner in parts.iter().rev() {
                stack.push(inner);
            }
        }
    }

    Err(OscalError::TooMany)
}

/// The label of an object of OSCAL (its prop `label`), if it has one:
/// `Some("ADV_ARC.1-1")`.
///
/// Input: `object`, a control or a part.
/// Output: its label, `None` when it has none.
///
/// Called by: [`part_text`], [`control_of`].
fn label_of(object: &Value) -> Option<&str> {
    for prop in object.get("props").and_then(Value::as_array)? {
        if prop.get("name").and_then(Value::as_str) == Some("label") {
            return prop.get("value").and_then(Value::as_str);
        }
    }

    None
}

/// The text of the parts named `name` of a control, joined.
///
/// Input: `control`, a control; `name`, `"statement"` or `"guidance"`.
/// Output: their text, the parts parted by a line break; empty when it has
/// none; the errors of [`part_text`].
///
/// Called by: [`control_of`].
/// Calls: [`part_text`].
fn parts_named(control: &Value, name: &str) -> Result<String, OscalError> {
    let mut texts = Vec::new();

    for part in control.get("parts").and_then(Value::as_array).map(Vec::as_slice).unwrap_or_default() {
        if part.get("name").and_then(Value::as_str) == Some(name) {
            texts.push(part_text(part)?);
        }
    }

    Ok(texts.join("\n"))
}

/// The text of the parts of a control that are neither its statement nor
/// its guidance, each under its title, or its name when it has none: what a
/// standard that names no part `guidance` says about the work beside the
/// requirement (the test units, the method of the assessment and the
/// assignment of the verdict of a test case of ETSI TS 103 701).
///
/// Input: `control`, a control.
/// Output: their text, the parts in the order of the control, parted by an
/// empty line ("test-units:\na) For each ...\n\nassessment-method:\nThe TL
/// examines ..."); empty when it has none; the errors of [`part_text`].
///
/// Called by: [`control_of`].
/// Calls: [`part_text`].
fn other_parts(control: &Value) -> Result<String, OscalError> {
    let mut texts = Vec::new();

    for part in control.get("parts").and_then(Value::as_array).map(Vec::as_slice).unwrap_or_default() {
        let name = part.get("name").and_then(Value::as_str).unwrap_or_default();
        let text = part_text(part)?;

        if name == "statement" || name == "guidance" || text.trim().is_empty() {
            continue;
        }

        let title = part.get("title").and_then(Value::as_str).unwrap_or(name);

        texts.push(format!("{title}:\n{text}"));
    }

    Ok(texts.join("\n\n"))
}

/// The control of the catalog as a work unit is made from it.
///
/// Input: `control`, a control of the catalog.
/// Output: the control (`CatalogControl { oscal_key: "adv_arc.1-1", label:
/// "ADV_ARC.1-1", .. }`); `OscalError::NotCatalog` for a control without
/// its id, the errors of [`parts_named`].
///
/// Called by: [`catalog_controls`].
/// Calls: [`label_of`], [`parts_named`], [`other_parts`].
fn control_of(control: &Value) -> Result<CatalogControl, OscalError> {
    let id = control.get("id").and_then(Value::as_str).unwrap_or_default();
    let title = control.get("title").and_then(Value::as_str).unwrap_or_default();
    let statement = parts_named(control, "statement")?;
    let guidance = parts_named(control, "guidance")?;

    ensure!(!id.is_empty(), OscalError::NotCatalog(vec!["a control without its id".to_owned()]));

    Ok(CatalogControl {
        oscal_key: id.to_owned(),
        label: label_of(control).unwrap_or(id).to_owned(),
        requirement: if statement.is_empty() { title.to_owned() } else { statement },
        // A standard that names its guidance its own way still gives it.
        guidance: if guidance.is_empty() { other_parts(control)? } else { guidance },
    })
}

/// Every control of the catalog `document`, in the order of the document:
/// those of its groups (and of the groups inside them) and the controls
/// inside controls (the enhancements of NIST).
///
/// Input: `document`, a catalog that passed [`check_catalog`].
/// Output: its controls (`[CatalogControl { .. }, ...]`);
/// `OscalError::NotCatalog` without a catalog, `OscalError::TooMany` for
/// more than [`MAX_OBJECTS`] objects.
///
/// Called by: `objects/evaluation/evaluation_plan/evaluation_plan_rules/work_units.rs`: `controls_of`.
/// Calls: [`push_items`], [`control_of`].
pub fn catalog_controls(document: &Value) -> Result<Vec<CatalogControl>, OscalError> {
    let catalog = document.get("catalog").ok_or_else(|| OscalError::NotCatalog(vec!["no \"catalog\"".to_owned()]))?;
    let mut stack: Vec<(Kind, &Value)> = Vec::new();
    let mut controls = Vec::new();

    push_items(&mut stack, catalog, "controls", Kind::Control);
    push_items(&mut stack, catalog, "groups", Kind::Group);

    for _ in 0..MAX_OBJECTS {
        let Some((kind, current)) = stack.pop() else {
            return Ok(controls);
        };

        if kind == Kind::Control {
            controls.push(control_of(current)?);
        }

        // A group holds groups and controls; a control holds controls. They
        // are pushed so that the controls of an object come before its groups.
        push_items(&mut stack, current, "groups", Kind::Group);
        push_items(&mut stack, current, "controls", Kind::Control);
    }

    Err(OscalError::TooMany)
}

/// Imports the OSCAL catalog `text`: checked against the schema of NIST,
/// stored as it was given, with the title and the version of its metadata.
///
/// Input: `text`, the document read from the file chosen.
/// Output: the id of the standard (`2`); the errors of [`check_catalog`],
/// `DomainError::Refused` for a catalog without a title, an error of the
/// database.
///
/// Called by: `panels/views/evaluation_plan/standards.rs`: `standard_import`.
/// Calls: [`check_catalog`]; `objects/evaluation/oscal_standard_document/oscal_standard_document_store.rs`:
/// `create_oscal_standard_document`.
pub fn import_standard(db: &UserDatabase, text: &str) -> Result<i64, DomainError> {
    let (_document, checked) = check_catalog(text)?;

    ensure!(!checked.title.is_empty(), DomainError::Refused("the catalog has no title".to_owned()));

    let standard =
        OscalStandardDocument { id: 0, title: checked.title, doc: text.to_owned(), version: checked.version };

    Ok(create_oscal_standard_document(db, &standard)?)
}

/// The fields of a standard a report writes (`{{oscal_standard_document.title}}`), in
/// the order of [`oscal_standard_document_fields`]; its catalog is no text of a report.
pub const OSCAL_STANDARD_DOCUMENT_FIELDS: [&str; 3] = ["id", "title", "version"];

/// The fields of the standard `document` as a report writes them, in the
/// order of [`OSCAL_STANDARD_DOCUMENT_FIELDS`].
///
/// Input: `document`, a standard of an evaluation.
/// Output: its fields (`["2", "Common Evaluation Methodology", "3.1R5"]`).
///
/// Called by: `objects/evaluation/report_template/report_template_rules/filling.rs`: `lists_of`.
pub fn oscal_standard_document_fields(document: &OscalStandardDocument) -> [String; 3] {
    [document.id.to_string(), document.title.clone(), document.version.clone()]
}

#[cfg(test)]
mod tests {
    //! Tests of the catalogs: the check against the schema of NIST, the
    //! controls of the sample of NIST and of the catalog of the CEM; the
    //! import of a standard.

    use super::*;

    use crate::objects::evaluation::oscal_standard_document::oscal_standard_document_store::read_oscal_standard_document;
    use crate::test_support::database::database;

    const NIST_SAMPLE: &str = include_str!("../../../../oscal/examples/basic-catalog.json");

    #[test]
    fn the_sample_of_nist_is_a_catalog_with_its_controls_in_order() {
        let (document, checked) = check_catalog(NIST_SAMPLE).unwrap();
        let controls = catalog_controls(&document).unwrap();

        assert!(!checked.title.is_empty());
        assert_eq!(controls[0].oscal_key, "s1.1.1");
        assert_eq!(controls[0].label, "1.1.1");
        assert!(controls[0].requirement.starts_with("All information security responsibilities"));
        assert!(!controls[0].guidance.is_empty());
    }

    #[test]
    fn what_is_no_catalog_is_refused() {
        assert!(matches!(check_catalog("not json"), Err(OscalError::NotJson(_))));
        assert!(matches!(check_catalog("{\"profile\": {}}"), Err(OscalError::NotCatalog(_))));
        assert!(matches!(check_catalog("{\"catalog\": {\"uuid\": \"x\"}}"), Err(OscalError::NotCatalog(_))));
    }

    #[test]
    fn the_work_units_of_the_cem_are_read_from_its_catalog() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../testing_documents/CEM2022R1_work_units_catalog.json");
        let text = std::fs::read_to_string(path).unwrap();
        let (document, checked) = check_catalog(&text).unwrap();
        let controls = catalog_controls(&document).unwrap();

        assert_eq!(checked.version, "CEM:2022 Revision 1 (CCMB-2022-11-006)");
        assert_eq!(controls.len(), 813);
        assert_eq!(controls[0].oscal_key, "APE_INT.1-1");
        assert!(controls[0].requirement.starts_with("The evaluator shall check"));
    }

    #[test]
    fn a_control_without_guidance_gives_its_other_parts_under_their_names() {
        let path =
            concat!(env!("CARGO_MANIFEST_DIR"), "/../testing_documents/ETSI_TS103701_V2.1.1_test_cases_catalog.json");
        let text = std::fs::read_to_string(path).unwrap();
        let (document, _) = check_catalog(&text).unwrap();
        let controls = catalog_controls(&document).unwrap();
        let case = controls.iter().find(|control| control.oscal_key == "tc-6-8-1").unwrap();

        assert_eq!(controls.len(), 131);
        assert!(case.requirement.starts_with("The purpose of this test case"));
        assert!(case.guidance.starts_with("test-units:\na) For each personal data in IXIT 21-PersData"));
        assert!(case.guidance.contains("\n\nassignment-of-verdict:\nThe verdict PASS is assigned if"));
        assert!(case.guidance.contains("\n\nassessment-method:\nConceptual assessment:"));
        assert!(!case.guidance.contains("The purpose of this test case"));

        // A control with a guidance keeps it alone, and one with neither has none.
        let with = serde_json::json!({"id": "c1", "parts": [
            {"name": "statement", "prose": "S"}, {"name": "guidance", "prose": "G"}, {"name": "objective", "prose": "O"}
        ]});
        let bare = serde_json::json!({"id": "c2", "title": "T", "parts": [{"name": "overview", "title": "Overview"}]});

        assert_eq!(control_of(&with).unwrap().guidance, "G");
        assert_eq!(control_of(&bare).unwrap().guidance, "");
    }

    #[test]
    fn a_catalog_is_imported_as_it_was_given() {
        let (_directory, db) = database();
        let id = import_standard(&db, NIST_SAMPLE).unwrap();
        let stored = read_oscal_standard_document(&db, id).unwrap();

        assert_eq!(stored.doc, NIST_SAMPLE);
        assert!(!stored.title.is_empty());
    }

    #[test]
    fn a_document_that_is_no_catalog_is_not_stored() {
        let (_directory, db) = database();

        assert!(matches!(import_standard(&db, "{\"catalog\": {}}"), Err(DomainError::Oscal(_))));
        assert!(matches!(import_standard(&db, "[]"), Err(DomainError::Oscal(_))));
    }
}
