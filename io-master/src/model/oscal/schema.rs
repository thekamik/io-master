//! The check of an OSCAL document against the JSON schema of NIST, of
//! the version the program follows (`OSCAL_VERSION`). The schemas are
//! the files of NIST in `oscal/schema/v1.2.3/`, built into the program;
//! nothing is fetched. A document that breaks its schema is never
//! stored and never exported.

use std::sync::OnceLock;

use jsonschema::Validator;
use serde_json::Value;

use super::{NotOscal, OscalModel};

/// How many messages of a failed check are kept for the evaluator.
const MAX_MESSAGES: usize = 10;

/// The schema of a model, as NIST publishes it.
fn schema_text(model: OscalModel) -> &'static str {
    match model {
        OscalModel::Catalog => include_str!("../../../oscal/schema/v1.2.3/oscal_catalog_schema.json"),
        OscalModel::Profile => include_str!("../../../oscal/schema/v1.2.3/oscal_profile_schema.json"),
        OscalModel::Mapping => include_str!("../../../oscal/schema/v1.2.3/oscal_mapping_schema.json"),
        OscalModel::AssessmentPlan => include_str!("../../../oscal/schema/v1.2.3/oscal_assessment-plan_schema.json"),
        OscalModel::AssessmentResults => {
            include_str!("../../../oscal/schema/v1.2.3/oscal_assessment-results_schema.json")
        }
    }
}

/// The validator of a model, compiled once.
fn validator(model: OscalModel) -> &'static Validator {
    static VALIDATORS: [OnceLock<Validator>; 5] =
        [OnceLock::new(), OnceLock::new(), OnceLock::new(), OnceLock::new(), OnceLock::new()];

    let index = OscalModel::ALL.iter().position(|known| *known == model).expect("every model is in ALL");

    VALIDATORS[index].get_or_init(|| {
        let schema: Value = serde_json::from_str(schema_text(model)).expect("the schema of NIST is JSON");

        jsonschema::validator_for(&schema).expect("the schema of NIST compiles")
    })
}

/// The model of a document: the one key of its root (a key `$schema`
/// aside).
pub fn model_of(document: &Value) -> Result<OscalModel, NotOscal> {
    let keys: Vec<&String> =
        document.as_object().map(|root| root.keys().filter(|key| *key != "$schema").collect()).unwrap_or_default();

    match keys.as_slice() {
        [key] => OscalModel::from_name(key).ok_or_else(|| NotOscal::UnknownModel(format!("root \"{key}\""))),
        _ => Err(NotOscal::UnknownModel("the root is not one object with one key".to_owned())),
    }
}

/// Checks a document against the schema of its model; returns the
/// model.
pub fn check(document: &Value) -> Result<OscalModel, NotOscal> {
    let model = model_of(document)?;
    let messages: Vec<String> = validator(model)
        .iter_errors(document)
        .take(MAX_MESSAGES)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect();

    if messages.is_empty() { Ok(model) } else { Err(NotOscal::Schema(messages)) }
}
