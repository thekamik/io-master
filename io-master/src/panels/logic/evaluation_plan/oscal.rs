//! The OSCAL catalogs of the standards, read for the plan: a document
//! checked against the JSON schema of NIST before it is stored (user,
//! 2026-10-28: a file of its own, not a part of `db/`), and the controls
//! of a catalog, from which the work units of a plan are made.
//!
//! The schema is the file of NIST in `oscal/schema/v1.2.3/`, built into
//! the program; nothing is fetched. The tree of a catalog (groups in
//! groups, controls in controls, parts in parts) is walked with an explicit
//! stack bounded by [`MAX_OBJECTS`], never by recursion (rule 1).

use jsonschema::Validator;
use serde_json::Value;

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
    /// The text of its part `guidance` (with the parts inside it); empty
    /// when it has none.
    pub guidance: String,
}

/// The validator of the schema of NIST, compiled from the text built into
/// the program.
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
/// Called by: [`check_catalog`].
fn metadata_text(document: &Value, field: &str) -> String {
    document.pointer(&format!("/catalog/metadata/{field}")).and_then(Value::as_str).unwrap_or_default().to_owned()
}

/// Checks that `text` is an OSCAL catalog: JSON within its limit, valid
/// against the schema of NIST. Gives the document read and its title and
/// version.
///
/// Called by: `standards.rs` (`import_standard`).
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

/// The label of an object of OSCAL (its prop `label`), if it has one.
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

/// The control of the catalog as a work unit is made from it.
///
/// Called by: [`catalog_controls`].
/// Calls: [`label_of`], [`parts_named`].
fn control_of(control: &Value) -> Result<CatalogControl, OscalError> {
    let id = control.get("id").and_then(Value::as_str).unwrap_or_default();
    let title = control.get("title").and_then(Value::as_str).unwrap_or_default();
    let statement = parts_named(control, "statement")?;

    ensure!(!id.is_empty(), OscalError::NotCatalog(vec!["a control without its id".to_owned()]));

    Ok(CatalogControl {
        oscal_key: id.to_owned(),
        label: label_of(control).unwrap_or(id).to_owned(),
        requirement: if statement.is_empty() { title.to_owned() } else { statement },
        guidance: parts_named(control, "guidance")?,
    })
}

/// Every control of the catalog `document`, in the order of the document:
/// those of its groups (and of the groups inside them) and the controls
/// inside controls (the enhancements of NIST).
///
/// Called by: `work_units.rs` (`add_work_unit`, `add_all_work_units`).
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

#[cfg(test)]
mod tests {
    //! Tests of the catalogs: the check against the schema of NIST, the
    //! controls of the sample of NIST and of the catalog of the CEM.

    use super::*;

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
}
