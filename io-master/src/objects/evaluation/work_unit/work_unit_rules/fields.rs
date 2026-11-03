//! The fields of a work unit as a report writes them (user, 2026-11-03):
//! each field of the model is the value of its marker
//! (`{{work_unit.findings}}`); a field that names other objects by their ids
//! writes their names, so that a report reads without the database.
//!
//! ```text
//! a work unit, the work units, standards, documents and proofs of its evaluation
//!   ──► work_unit_fields ──► [the values, in the order of WORK_UNIT_FIELDS]
//! ```

use crate::infrastructure::storage::error::DbError;
use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::ai::agent::agent_store::read_agent;
use crate::objects::error::DomainError;
use crate::objects::evaluation::lab_proof::lab_proof_model::LabProof;
use crate::objects::evaluation::oscal_standard_document::oscal_standard_document_model::OscalStandardDocument;
use crate::objects::evaluation::product_document::product_document_model::ProductDocument;
use crate::objects::evaluation::work_unit::work_unit_model::{Author, ProofQuote, Quote, Verdict, WorkUnit};

/// The fields of a work unit a report writes, in the order of
/// [`work_unit_fields`]: every field of the model.
pub const WORK_UNIT_FIELDS: [&str; 19] = [
    "id",
    "reference",
    "label",
    "oscal_key",
    "requirement",
    "guidance",
    "dependencies",
    "cross_reference",
    "input_product_documents",
    "input_functional",
    "ai_agent",
    "findings",
    "analysis",
    "vulnerability_notes",
    "notes",
    "quotes",
    "proof_quotes",
    "verdict",
    "author",
];

/// The objects of an evaluation a work unit names by their ids, read once
/// for the whole report.
pub struct Named<'a> {
    /// The work units of the evaluation (its dependencies).
    pub units: &'a [WorkUnit],
    /// The standards of the evaluation (its reference).
    pub standards: &'a [OscalStandardDocument],
    /// The documents of the product of the evaluation.
    pub documents: &'a [ProductDocument],
    /// The proofs of the laboratory its work units need.
    pub proofs: &'a [LabProof],
}

/// A verdict as a report writes it (`Verdict::NotApplicable` → "Not applicable").
///
/// Called by: [`work_unit_fields`];
/// `objects/evaluation/evaluation_plan/evaluation_plan_rules/fields.rs`: `evaluation_plan_fields`.
pub fn verdict_text(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Pass => "Pass",
        Verdict::Fail => "Fail",
        Verdict::Inconclusive => "Inconclusive",
        Verdict::NotApplicable => "Not applicable",
    }
}

/// The error of an id a work unit names that is not among the objects of
/// its evaluation: the database holds what the program never writes.
///
/// Called by: [`label_of`], [`standard_title`], [`document_name`], [`proof_name`].
fn missing(what: &str, id: i64) -> DomainError {
    DomainError::Db(DbError::Inconsistent(format!("a work unit names {what} {id}, not one of its evaluation")))
}

/// The label of the work unit `id` among `units` ("ADV_ARC.1-1").
///
/// Called by: [`labels`].
/// Calls: [`missing`].
fn label_of(units: &[WorkUnit], id: i64) -> Result<&str, DomainError> {
    for unit in units {
        if unit.id == id {
            return Ok(&unit.label);
        }
    }

    Err(missing("the work unit", id))
}

/// The title of the standard `id` among `standards` ("Common Evaluation Methodology").
///
/// Called by: [`work_unit_fields`].
/// Calls: [`missing`].
fn standard_title(standards: &[OscalStandardDocument], id: i64) -> Result<String, DomainError> {
    for standard in standards {
        if standard.id == id {
            return Ok(standard.title.clone());
        }
    }

    Err(missing("the standard", id))
}

/// The name of the document `id` among `documents` ("ST").
///
/// Called by: [`document_names`], [`quotes_text`].
/// Calls: [`missing`].
fn document_name(documents: &[ProductDocument], id: i64) -> Result<&str, DomainError> {
    for document in documents {
        if document.id == id {
            return Ok(&document.name);
        }
    }

    Err(missing("the document", id))
}

/// The name of the proof `id` among `proofs` ("Scan").
///
/// Called by: [`proof_names`], [`proof_quotes_text`].
/// Calls: [`missing`].
fn proof_name(proofs: &[LabProof], id: i64) -> Result<&str, DomainError> {
    for proof in proofs {
        if proof.id == id {
            return Ok(&proof.name);
        }
    }

    Err(missing("the proof", id))
}

/// The labels of the dependencies `ids`, parted by commas ("ADV_ARC.1-1, ADV_FSP.4-1").
///
/// Called by: [`work_unit_fields`].
/// Calls: [`label_of`].
fn labels(units: &[WorkUnit], ids: &[i64]) -> Result<String, DomainError> {
    let mut found = Vec::with_capacity(ids.len());

    for id in ids {
        found.push(label_of(units, *id)?);
    }

    Ok(found.join(", "))
}

/// The names of the documents `ids`, parted by commas ("ST, IXIT").
///
/// Called by: [`work_unit_fields`].
/// Calls: [`document_name`].
fn document_names(documents: &[ProductDocument], ids: &[i64]) -> Result<String, DomainError> {
    let mut found = Vec::with_capacity(ids.len());

    for id in ids {
        found.push(document_name(documents, *id)?);
    }

    Ok(found.join(", "))
}

/// The names of the proofs `ids`, parted by commas ("Scan, Fuzzing").
///
/// Called by: [`work_unit_fields`].
/// Calls: [`proof_name`].
fn proof_names(proofs: &[LabProof], ids: &[i64]) -> Result<String, DomainError> {
    let mut found = Vec::with_capacity(ids.len());

    for id in ids {
        found.push(proof_name(proofs, *id)?);
    }

    Ok(found.join(", "))
}

/// The quotes `quotes`, one in a line, each after the name of its document
/// (`ST: "The TOE is the router R7"`).
///
/// Called by: [`work_unit_fields`].
/// Calls: [`document_name`].
fn quotes_text(documents: &[ProductDocument], quotes: &[Quote]) -> Result<String, DomainError> {
    let mut lines = Vec::with_capacity(quotes.len());

    for quote in quotes {
        lines.push(format!("{}: \"{}\"", document_name(documents, quote.product_document)?, quote.text));
    }

    Ok(lines.join("\n"))
}

/// The quotes of the proofs `quotes`, one in a line, each after the name of
/// its proof (`Scan: "22/tcp open ssh"`).
///
/// Called by: [`work_unit_fields`].
/// Calls: [`proof_name`].
fn proof_quotes_text(proofs: &[LabProof], quotes: &[ProofQuote]) -> Result<String, DomainError> {
    let mut lines = Vec::with_capacity(quotes.len());

    for quote in quotes {
        lines.push(format!("{}: \"{}\"", proof_name(proofs, quote.lab_proof)?, quote.text));
    }

    Ok(lines.join("\n"))
}

/// The fields of the work unit `unit` as a report writes them, in the order
/// of [`WORK_UNIT_FIELDS`]: the names of what it names, "yes" or "no", a
/// verdict or an author not given empty.
///
/// Input: `db`, the open database of the user (the name of its agent);
/// `unit`, a work unit of an evaluation; `named`, the objects of its
/// evaluation.
/// Output: its fields (`["31", "CEM", "ADV_ARC.1-1", ..., "Pass", "AI"]`); an
/// error when it names an object not among `named`, or a missing agent.
///
/// Called by: `objects/evaluation/report_template/report_template_rules/filling.rs`: `lists_of`.
/// Calls: [`standard_title`], [`labels`], [`document_names`], [`proof_names`], [`quotes_text`],
/// [`proof_quotes_text`], [`verdict_text`]; `objects/ai/agent/agent_store.rs`: `read_agent`.
pub fn work_unit_fields(db: &UserDatabase, unit: &WorkUnit, named: &Named<'_>) -> Result<[String; 19], DomainError> {
    let agent = match unit.ai_agent {
        Some(agent) => read_agent(db, agent)?.name,
        None => String::new(),
    };
    let author = match unit.author {
        Some(Author::Ai) => "AI",
        Some(Author::User) => "evaluator",
        None => "",
    };

    Ok([
        unit.id.to_string(),
        standard_title(named.standards, unit.reference)?,
        unit.label.clone(),
        unit.oscal_key.clone(),
        unit.requirement.clone(),
        unit.guidance.clone(),
        labels(named.units, &unit.dependencies)?,
        (if unit.cross_reference { "yes" } else { "no" }).to_owned(),
        document_names(named.documents, &unit.input_product_documents)?,
        proof_names(named.proofs, unit.input_functional.as_deref().unwrap_or_default())?,
        agent,
        unit.findings.clone(),
        unit.analysis.clone(),
        unit.vulnerability_notes.clone(),
        unit.notes.clone(),
        quotes_text(named.documents, &unit.quotes)?,
        proof_quotes_text(named.proofs, &unit.proof_quotes)?,
        unit.verdict.map(verdict_text).unwrap_or_default().to_owned(),
        author.to_owned(),
    ])
}

#[cfg(test)]
mod tests {
    //! Tests of the fields of a work unit: the names of what it names, an id
    //! outside its evaluation refused.

    use super::*;

    use crate::test_support::database::database;

    fn standard() -> OscalStandardDocument {
        OscalStandardDocument { id: 2, title: "CEM".to_owned(), doc: String::new(), version: "3.1".to_owned() }
    }

    fn st() -> ProductDocument {
        ProductDocument { id: 4, name: "ST".to_owned(), content: None, version: None }
    }

    fn assessed() -> WorkUnit {
        WorkUnit {
            id: 31,
            reference: 2,
            label: "ASE_INT.1-1".to_owned(),
            oscal_key: "ase_int.1-1".to_owned(),
            requirement: "The evaluator shall check".to_owned(),
            guidance: String::new(),
            dependencies: Vec::new(),
            cross_reference: false,
            input_product_documents: vec![4],
            input_functional: None,
            ai_agent: None,
            findings: "none".to_owned(),
            analysis: String::new(),
            vulnerability_notes: String::new(),
            notes: String::new(),
            quotes: vec![Quote { product_document: 4, text: "The TOE".to_owned() }],
            proof_quotes: Vec::new(),
            verdict: Some(Verdict::NotApplicable),
            author: Some(Author::User),
        }
    }

    #[test]
    fn a_work_unit_writes_the_names_of_what_it_names() {
        let (_directory, db) = database();
        let units = [assessed()];
        let standards = [standard()];
        let documents = [st()];
        let named = Named { units: &units, standards: &standards, documents: &documents, proofs: &[] };
        let fields = work_unit_fields(&db, &units[0], &named).unwrap();

        assert_eq!(fields[1], "CEM");
        assert_eq!(fields[7], "no");
        assert_eq!(fields[8], "ST");
        assert_eq!(fields[15], "ST: \"The TOE\"");
        assert_eq!(fields[17], "Not applicable");
        assert_eq!(fields[18], "evaluator");

        let elsewhere = WorkUnit { dependencies: vec![99], ..assessed() };

        assert!(matches!(work_unit_fields(&db, &elsewhere, &named), Err(DomainError::Db(_))));
    }
}
