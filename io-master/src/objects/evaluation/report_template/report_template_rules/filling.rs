//! A report (user, 2026-11-03): a template filled with an evaluation, its
//! markers replaced by the fields of the evaluation, each block written
//! once for each element of its list (the work units in the order of the
//! work); written into a file the evaluator chooses.
//!
//! ```text
//! an evaluation, a template ──► filled_report
//!   the evaluation ──► evaluation_plan_fields ──► the values of {{evaluation_plan.*}}
//!   its standards, documents, proofs, work units ──► <object>_fields ──► the values of each element
//!   each value made harmless in the language of the file of the template (escaping.rs: escaped)
//!   segments(template) ──► report_text: a text replaced, a block for each element its filter takes
//!   ──► the report, at most MAX_REPORT bytes ──► write_report ──► the file
//! ```

use std::fs;
use std::path::Path;

use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::error::DomainError;
use crate::objects::evaluation::evaluation_plan::evaluation_plan_model::{EvaluationPlan, PlanState};
use crate::objects::evaluation::evaluation_plan::evaluation_plan_rules::evaluation::needed_proofs;
use crate::objects::evaluation::evaluation_plan::evaluation_plan_rules::fields::{
    EVALUATION_PLAN_FIELDS, evaluation_plan_fields,
};
use crate::objects::evaluation::evaluation_plan::evaluation_plan_rules::order::work_order;
use crate::objects::evaluation::evaluation_plan::evaluation_plan_store::read_evaluation_plan;
use crate::objects::evaluation::lab_proof::lab_proof_rules::lab_proof_fields;
use crate::objects::evaluation::lab_proof::lab_proof_store::read_lab_proof;
use crate::objects::evaluation::oscal_standard_document::oscal_standard_document_rules::oscal_standard_document_fields;
use crate::objects::evaluation::oscal_standard_document::oscal_standard_document_store::read_oscal_standard_document;
use crate::objects::evaluation::product_document::product_document_rules::product_document_fields;
use crate::objects::evaluation::product_document::product_document_store::read_document_record;
use crate::objects::evaluation::report_template::report_template_rules::escaping::{Escape, escape_of, escaped};
use crate::objects::evaluation::report_template::report_template_rules::markers::{
    EVALUATION_PLAN, Filter, List, Segment, element_model, marker, replaced, segments,
};
use crate::objects::evaluation::report_template::report_template_store::read_report_template;
use crate::objects::evaluation::work_unit::work_unit_model::{Verdict, WorkUnit};
use crate::objects::evaluation::work_unit::work_unit_rules::fields::{Named, work_unit_fields};
use crate::objects::evaluation::work_unit::work_unit_store::read_work_units;

/// The largest report, in bytes.
pub const MAX_REPORT: usize = 32 * 1024 * 1024;

/// An element of a list of an evaluation, as a block writes it.
struct Element {
    /// Its verdict, for the filter of a block of work units; `None` for a
    /// work unit not assessed and for an element of another list.
    verdict: Option<Verdict>,
    /// The markers of its fields and of those of the evaluation, with their values.
    values: Vec<(String, String)>,
}

/// The lists of an evaluation, each element with the values of its markers.
struct Lists {
    oscal_documents: Vec<Element>,
    product_documents: Vec<Element>,
    lab_proofs: Vec<Element>,
    work_units: Vec<Element>,
}

/// The markers of the fields `fields` of the model `model` with their
/// values `values`, each field with the value at its place, written as the
/// file of the report holds a text (`escape`).
///
/// Input: `model`, the name of a model (`"work_unit"`); `fields`, its fields;
/// `values`, their values in the same order; `escape`, how the report writes
/// a value.
/// Output: the pairs (`[("{{work_unit.id}}", "31"), ...]`).
///
/// Called by: [`filled_report`], [`element_of`].
/// Calls: `objects/evaluation/report_template/report_template_rules/markers.rs`: `marker`;
/// `objects/evaluation/report_template/report_template_rules/escaping.rs`: `escaped`.
fn values_of<const N: usize>(
    model: &str,
    fields: &[&str; N],
    values: [String; N],
    escape: Escape,
) -> Vec<(String, String)> {
    let mut pairs = Vec::with_capacity(N);

    for (field, value) in fields.iter().zip(values) {
        pairs.push((marker(model, field), escaped(value, escape)));
    }

    pairs
}

/// An element of the list `list` with the values `values` of its fields,
/// written as the file of the report holds a text (`escape`). The values of
/// the evaluation are not copied into each element: [`report_text`] gives
/// them beside it.
///
/// Called by: [`lists_of`].
/// Calls: `objects/evaluation/report_template/report_template_rules/markers.rs`: `element_model`; [`values_of`].
fn element_of<const N: usize>(
    list: List,
    verdict: Option<Verdict>,
    values: [String; N],
    escape: Escape,
) -> Result<Element, DomainError> {
    let (model, fields) = element_model(list);
    let Ok(fields) = <&[&str; N]>::try_from(fields) else {
        return Err(DomainError::Refused(format!("the fields of {model} are not {N}")));
    };

    Ok(Element { verdict, values: values_of(model, fields, values, escape) })
}

/// The work units `units` of an evaluation in the order of the work.
///
/// Input: `units`, the work units in the order of the plan.
/// Output: the work units in the order of the work (`[ASE_INT.1-1, ASE_INT.1-2]`), those of a
/// cycle at the end; an error of `work_order`.
///
/// Called by: [`filled_report`].
/// Calls: `objects/evaluation/evaluation_plan/evaluation_plan_rules/order.rs`: `work_order`.
fn units_in_order(units: &[WorkUnit]) -> Result<Vec<WorkUnit>, DomainError> {
    let order = work_order(units)?;
    let mut ordered = Vec::with_capacity(units.len());

    for position in order.order.iter().chain(&order.crossed) {
        let Some(unit) = units.get(*position) else {
            return Err(DomainError::Refused(format!("the order of the work names the position {position}")));
        };

        ordered.push(unit.clone());
    }

    ensure!(ordered.len() == units.len(), DomainError::Refused("the order of the work leaves work units".to_owned()));

    Ok(ordered)
}

/// The lists of the evaluation `plan`: its standards, its documents, the
/// proofs its work units need and its work units, each element with its
/// values.
///
/// Input: `db`, the open database of the user; `plan`, an evaluation;
/// `units`, its work units in the order of the work; `escape`, how the report
/// writes a value.
/// Output: the lists; an error when an object is missing.
///
/// Called by: [`filled_report`].
/// Calls: `objects/evaluation/oscal_standard_document/oscal_standard_document_store.rs`:
/// `read_oscal_standard_document`; `objects/evaluation/product_document/product_document_store.rs`:
/// `read_document_record`; `objects/evaluation/evaluation_plan/evaluation_plan_rules/evaluation.rs`:
/// `needed_proofs`; `objects/evaluation/lab_proof/lab_proof_store.rs`: `read_lab_proof`; the fields of
/// each (`oscal_standard_document_fields`, `product_document_fields`, `lab_proof_fields`,
/// `work_unit_fields`); [`element_of`].
fn lists_of(
    db: &UserDatabase,
    plan: &EvaluationPlan,
    units: &[WorkUnit],
    escape: Escape,
) -> Result<Lists, DomainError> {
    let mut standards = Vec::with_capacity(plan.oscal_documents.len());
    let mut documents = Vec::with_capacity(plan.product_documents.len());
    let proof_ids = needed_proofs(units);
    let mut proofs = Vec::with_capacity(proof_ids.len());

    for id in &plan.oscal_documents {
        standards.push(read_oscal_standard_document(db, *id)?);
    }
    for id in &plan.product_documents {
        documents.push(read_document_record(db, *id)?.0);
    }
    for id in proof_ids {
        proofs.push(read_lab_proof(db, id)?);
    }

    let mut lists = Lists {
        oscal_documents: Vec::with_capacity(standards.len()),
        product_documents: Vec::with_capacity(documents.len()),
        lab_proofs: Vec::with_capacity(proofs.len()),
        work_units: Vec::with_capacity(units.len()),
    };

    for standard in &standards {
        let values = oscal_standard_document_fields(standard);

        lists.oscal_documents.push(element_of(List::OscalDocuments, None, values, escape)?);
    }
    for document in &documents {
        lists.product_documents.push(element_of(
            List::ProductDocuments,
            None,
            product_document_fields(document),
            escape,
        )?);
    }
    for proof in &proofs {
        lists.lab_proofs.push(element_of(List::LabProofs, None, lab_proof_fields(proof), escape)?);
    }

    let named = Named { units, standards: &standards, documents: &documents, proofs: &proofs };

    for unit in units {
        let values = work_unit_fields(db, unit, &named)?;

        lists.work_units.push(element_of(List::WorkUnits, unit.verdict, values, escape)?);
    }

    Ok(lists)
}

/// Adds the text `text` to the report `report`, refused past [`MAX_REPORT`].
///
/// Called by: [`report_text`].
fn push_bounded(report: &mut String, text: &str) -> Result<(), DomainError> {
    ensure!(
        report.len().saturating_add(text.len()) <= MAX_REPORT,
        DomainError::Refused(format!("the report over {MAX_REPORT} bytes"))
    );

    report.push_str(text);
    Ok(())
}

/// The report of the parts `parts` of a template: each text with the
/// markers of the evaluation replaced, each block written for every element
/// of its list its filter takes, with the markers of the element and of the
/// evaluation replaced.
///
/// Input: `parts`, the parts of a template; `plan_values`, the values of the
/// markers of the evaluation; `lists`, its lists.
/// Output: the report ("# R7\n- ASE_INT.1-1: Pass\n"); `DomainError::Refused`
/// past [`MAX_REPORT`].
///
/// Called by: [`filled_report`].
/// Calls: `objects/evaluation/report_template/report_template_rules/markers.rs`: `replaced`; [`push_bounded`].
fn report_text(parts: &[Segment<'_>], plan_values: &[(String, String)], lists: &Lists) -> Result<String, DomainError> {
    let mut report = String::new();

    for part in parts {
        let (list, filter, body) = match *part {
            Segment::Text(text) => {
                push_bounded(&mut report, &replaced(text, plan_values, &[]))?;
                continue;
            }
            Segment::Block { list, filter, body } => (list, filter, body),
        };
        let elements = match list {
            List::OscalDocuments => &lists.oscal_documents,
            List::ProductDocuments => &lists.product_documents,
            List::LabProofs => &lists.lab_proofs,
            List::WorkUnits => &lists.work_units,
        };

        for element in elements {
            if filter == Filter::All || filter == Filter::Verdict(element.verdict) {
                push_bounded(&mut report, &replaced(body, &element.values, plan_values))?;
            }
        }
    }

    Ok(report)
}

/// The report of the evaluation `evaluation` written by the template
/// `template`.
///
/// Input: `db`, the open database of the user; `evaluation`, the id of an
/// evaluation (in evaluation or a report); `template`, the id of a template.
/// Output: the report ("# Router R7\n..."), its values written as the
/// language of the extension of the template holds a text (HTML, LaTeX;
/// `escaping.rs`); `DomainError::Refused` for a plan
/// in planning, a template whose blocks are wrong, a report past
/// [`MAX_REPORT`]; an error of the database.
///
/// Called by: [`write_report`]; `panels/views/reports/report.rs`: `report_preview`.
/// Calls: `objects/evaluation/evaluation_plan/evaluation_plan_store.rs`: `read_evaluation_plan`;
/// `objects/evaluation/report_template/report_template_store.rs`: `read_report_template`;
/// `objects/evaluation/report_template/report_template_rules/markers.rs`: `segments`;
/// `objects/evaluation/work_unit/work_unit_store.rs`: `read_work_units`;
/// `objects/evaluation/evaluation_plan/evaluation_plan_rules/fields.rs`: `evaluation_plan_fields`;
/// `objects/evaluation/report_template/report_template_rules/escaping.rs`: `escape_of`;
/// [`values_of`], [`units_in_order`], [`lists_of`], [`report_text`].
pub fn filled_report(db: &UserDatabase, evaluation: i64, template: i64) -> Result<String, DomainError> {
    let plan = read_evaluation_plan(db, evaluation)?;

    ensure!(plan.state != PlanState::Plan, DomainError::Refused(format!("the plan {} is no evaluation", plan.name)));

    let template = read_report_template(db, template)?;
    let parts = segments(&template.text)?;
    let units = units_in_order(&read_work_units(db, evaluation)?)?;
    // A value is a text of the evaluation, never a part of the template: it
    // is written as the language of the file holds a text.
    let escape = escape_of(&template.extension);
    let plan_values = values_of(EVALUATION_PLAN, &EVALUATION_PLAN_FIELDS, evaluation_plan_fields(db, &plan)?, escape);
    let lists = lists_of(db, &plan, &units, escape)?;

    report_text(&parts, &plan_values, &lists)
}

/// Writes the report of the evaluation `evaluation` by the template
/// `template` into the file `path`, replacing it.
///
/// Input: `db`, the open database of the user; `evaluation`, `template`, as
/// [`filled_report`]; `path`, the file chosen in the dialog of the page.
/// Output: nothing; the errors of [`filled_report`], `DomainError::Refused`
/// when the file cannot be written.
///
/// Called by: `panels/views/reports/report.rs`: `report_export`.
/// Calls: [`filled_report`].
pub fn write_report(db: &UserDatabase, evaluation: i64, template: i64, path: &Path) -> Result<(), DomainError> {
    let report = filled_report(db, evaluation, template)?;

    fs::write(path, report).map_err(|error| DomainError::Refused(format!("{}: {error}", path.display())))
}

#[cfg(test)]
mod tests {
    //! Tests of a report: the markers of the evaluation and of its work units
    //! replaced, a block written for each element its filter takes, in the
    //! order of the work; a value kept as text in a report of HTML; a plan in
    //! planning has none.

    use super::*;

    use crate::objects::evaluation::evaluation_plan::evaluation_plan_store::create_evaluation_plan;
    use crate::objects::evaluation::report_template::report_template_model::ReportTemplate;
    use crate::objects::evaluation::report_template::report_template_store::create_report_template;
    use crate::objects::evaluation::work_unit::work_unit_store::read_work_unit;
    use crate::test_support::TempDir;
    use crate::test_support::database::{database, plan};
    use crate::test_support::evaluation::{evaluation, write};

    /// Stores a template of the text `text` and gives its id.
    fn template(db: &UserDatabase, text: &str) -> i64 {
        let template =
            ReportTemplate { id: 0, name: "ETR".to_owned(), extension: "md".to_owned(), text: text.to_owned() };

        create_report_template(db, &template).unwrap()
    }

    #[test]
    fn a_report_writes_the_evaluation_and_its_work_units_in_order() {
        let (_directory, db) = database();
        let (id, first, second) = evaluation(&db);

        write(
            &db,
            &WorkUnit {
                verdict: Some(Verdict::Fail),
                findings: "no TOE".to_owned(),
                ..read_work_unit(&db, first).unwrap()
            },
        );

        let text = "# {{evaluation_plan.product}} for {{evaluation_plan.client}}\n\
                    {{#evaluation_plan.product_documents}}{{product_document.name}} {{product_document.version}}\n\
                    {{/evaluation_plan.product_documents}}\
                    {{#evaluation_plan.work_units}}- {{work_unit.label}}: {{work_unit.verdict}} ({{evaluation_plan.product}})\n\
                    {{/evaluation_plan.work_units}}\
                    {{#evaluation_plan.work_units verdict=fail}}! {{work_unit.label}} {{work_unit.findings}}\n\
                    {{/evaluation_plan.work_units}}{{unknown}}";
        let report = filled_report(&db, id, template(&db, text)).unwrap();
        let second = read_work_unit(&db, second).unwrap();

        assert_eq!(
            report,
            format!(
                "# R7 for ACME\nST 1.0\n- ASE_INT.1-1: Fail (R7)\n- {}:  (R7)\n! ASE_INT.1-1 no TOE\n{{{{unknown}}}}",
                second.label
            )
        );
    }

    #[test]
    fn a_value_stays_text_in_a_report_of_html() {
        let (_directory, db) = database();
        let (id, first, _) = evaluation(&db);

        write(
            &db,
            &WorkUnit {
                verdict: Some(Verdict::Fail),
                findings: "<script>alert(1)</script>".to_owned(),
                ..read_work_unit(&db, first).unwrap()
            },
        );

        let page = ReportTemplate {
            id: 0,
            name: "ETR page".to_owned(),
            extension: "html".to_owned(),
            text: "{{#evaluation_plan.work_units verdict=fail}}<p>{{work_unit.findings}}</p>{{/evaluation_plan.work_units}}"
                .to_owned(),
        };
        let report = filled_report(&db, id, create_report_template(&db, &page).unwrap()).unwrap();

        // The template is HTML, the value only its text.
        assert_eq!(report, "<p>&lt;script&gt;alert(1)&lt;/script&gt;</p>");
    }

    #[test]
    fn a_report_is_written_into_its_file() {
        let (_directory, db) = database();
        let (id, _, _) = evaluation(&db);
        let folder = TempDir::new();
        let path = folder.path().join("report.md");

        write_report(&db, id, template(&db, "{{evaluation_plan.evaluator}}"), &path).unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "K. K.");
    }

    #[test]
    fn a_plan_in_planning_has_no_report() {
        let (_directory, db) = database();
        let routers = create_evaluation_plan(&db, &plan(&db)).unwrap();

        assert!(matches!(filled_report(&db, routers, template(&db, "x")), Err(DomainError::Refused(_))));
    }
}
