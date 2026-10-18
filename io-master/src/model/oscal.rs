//! The documents of NIST OSCAL (Open Security Controls Assessment
//! Language), version 1.2.3: the format in which the program keeps the
//! standards and the plans of evaluations (step 41 of docs/todo.md,
//! docs/oscal.md).
//!
//! ```text
//! common.rs              metadata, properties, links, parts, parameters, back matter
//! catalog.rs             a catalog: the requirements (controls) of a standard
//! mapping.rs             a mapping collection: relations between requirements
//! profile.rs             a profile: a selection of requirements, parameters set
//! assessment_plan.rs     an assessment plan: the evaluation plan
//! assessment_results.rs  assessment results: the evaluation
//! schema.rs              the check of a document against the JSON schema of NIST
//! values.rs              UUIDs, dates and tokens as OSCAL defines them
//! ```
//!
//! **An exception to the rule "the core types carry no serialization
//! code"** (decided by the user, 2026-10-18): these types *are* a
//! published format, so they carry serde with the names of OSCAL
//! (`kebab-case`), and a document becomes its JSON without a conversion
//! written by hand. Only the fields the program works with are typed;
//! every other field of a document is kept as it came (`other`), so a
//! file of NIST passes through the program unchanged.
//!
//! What the program adds to a document (instructions to the AI, quotes
//! and their checks, the layout of a graph) is never written into it:
//! it is the evaluation support, kept apart and linked by UUIDs and ids.

pub mod assessment_plan;
pub mod assessment_results;
pub mod catalog;
pub mod common;
pub mod mapping;
pub mod profile;
pub mod schema;
pub mod values;

use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use assessment_plan::AssessmentPlan;
use assessment_results::AssessmentResults;
use catalog::Catalog;
use common::Metadata;
use mapping::MappingCollection;
use profile::Profile;

/// The version of OSCAL the program writes and checks against.
pub const OSCAL_VERSION: &str = "1.2.3";

/// The models of OSCAL the program works with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OscalModel {
    Catalog,
    Profile,
    Mapping,
    AssessmentPlan,
    AssessmentResults,
}

impl OscalModel {
    pub const ALL: [OscalModel; 5] = [
        OscalModel::Catalog,
        OscalModel::Profile,
        OscalModel::Mapping,
        OscalModel::AssessmentPlan,
        OscalModel::AssessmentResults,
    ];

    /// The name of the root of a document of the model, as OSCAL writes
    /// it (`{"catalog": {...}}`), also kept in the database.
    pub fn name(self) -> &'static str {
        match self {
            OscalModel::Catalog => "catalog",
            OscalModel::Profile => "profile",
            OscalModel::Mapping => "mapping-collection",
            OscalModel::AssessmentPlan => "assessment-plan",
            OscalModel::AssessmentResults => "assessment-results",
        }
    }

    pub fn from_name(name: &str) -> Option<OscalModel> {
        OscalModel::ALL.into_iter().find(|model| model.name() == name)
    }

    /// The name shown to the evaluator.
    pub fn title(self) -> &'static str {
        match self {
            OscalModel::Catalog => "Catalog",
            OscalModel::Profile => "Profile",
            OscalModel::Mapping => "Mapping",
            OscalModel::AssessmentPlan => "Assessment plan",
            OscalModel::AssessmentResults => "Assessment results",
        }
    }
}

/// A document of OSCAL: its JSON is the object with one key, the name
/// of its model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OscalDocument {
    Catalog(Catalog),
    Profile(Profile),
    MappingCollection(MappingCollection),
    AssessmentPlan(AssessmentPlan),
    AssessmentResults(AssessmentResults),
}

/// Why a JSON text is not a document of OSCAL the program can take.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotOscal {
    /// The text is not JSON; what the parser said.
    NotJson(String),
    /// The root is not one of the models of `OscalModel`.
    UnknownModel(String),
    /// The document breaks the schema of NIST; the messages of the check.
    Schema(Vec<String>),
    /// It keeps the schema but not the types of the program (a mistake
    /// of the program, reported as such).
    Types(String),
}

impl fmt::Display for NotOscal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotOscal::NotJson(reason) => write!(formatter, "the file is not JSON: {reason}"),
            NotOscal::UnknownModel(root) => write!(
                formatter,
                "the file is not an OSCAL catalog, profile, mapping, assessment plan or assessment results ({root})"
            ),
            NotOscal::Schema(messages) => {
                write!(
                    formatter,
                    "the document breaks the OSCAL schema {OSCAL_VERSION} of NIST: {}",
                    messages.join("; ")
                )
            }
            NotOscal::Types(reason) => write!(formatter, "the document could not be read: {reason}"),
        }
    }
}

impl std::error::Error for NotOscal {}

impl OscalDocument {
    /// Reads a document from its JSON: the model found from its root,
    /// the document checked against the schema of NIST, then read into
    /// the types of the program. A key `$schema` at the root is allowed
    /// and dropped.
    pub fn from_json(text: &str) -> Result<OscalDocument, NotOscal> {
        let mut value: Value = serde_json::from_str(text).map_err(|error| NotOscal::NotJson(error.to_string()))?;

        if let Some(root) = value.as_object_mut() {
            root.remove("$schema");
        }

        schema::check(&value)?;

        serde_json::from_value(value).map_err(|error| NotOscal::Types(error.to_string()))
    }

    /// The document as the JSON of OSCAL, checked against the schema of
    /// NIST: what the program stores and exports.
    pub fn to_json(&self) -> Result<Value, NotOscal> {
        let value = serde_json::to_value(self).map_err(|error| NotOscal::Types(error.to_string()))?;

        schema::check(&value)?;

        Ok(value)
    }

    pub fn model(&self) -> OscalModel {
        match self {
            OscalDocument::Catalog(_) => OscalModel::Catalog,
            OscalDocument::Profile(_) => OscalModel::Profile,
            OscalDocument::MappingCollection(_) => OscalModel::Mapping,
            OscalDocument::AssessmentPlan(_) => OscalModel::AssessmentPlan,
            OscalDocument::AssessmentResults(_) => OscalModel::AssessmentResults,
        }
    }

    pub fn uuid(&self) -> &str {
        match self {
            OscalDocument::Catalog(document) => &document.uuid,
            OscalDocument::Profile(document) => &document.uuid,
            OscalDocument::MappingCollection(document) => &document.uuid,
            OscalDocument::AssessmentPlan(document) => &document.uuid,
            OscalDocument::AssessmentResults(document) => &document.uuid,
        }
    }

    pub fn metadata(&self) -> &Metadata {
        match self {
            OscalDocument::Catalog(document) => &document.metadata,
            OscalDocument::Profile(document) => &document.metadata,
            OscalDocument::MappingCollection(document) => &document.metadata,
            OscalDocument::AssessmentPlan(document) => &document.metadata,
            OscalDocument::AssessmentResults(document) => &document.metadata,
        }
    }

    /// What a list of documents shows.
    pub fn summary(&self) -> OscalSummary {
        let metadata = self.metadata();

        OscalSummary {
            uuid: self.uuid().to_owned(),
            model: self.model(),
            title: metadata.title.clone(),
            version: metadata.version.clone(),
            oscal_version: metadata.oscal_version.clone(),
            last_modified: metadata.last_modified.clone(),
        }
    }
}

/// A document in a list: what identifies it, without its content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OscalSummary {
    pub uuid: String,
    pub model: OscalModel,
    pub title: String,
    pub version: String,
    pub oscal_version: String,
    /// As OSCAL writes it (RFC 3339 date and time).
    pub last_modified: String,
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "oscal_tests.rs"]
mod tests;
