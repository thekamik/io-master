//! The markers of a template (user, 2026-11-03): a field of a model in
//! double braces, named by the model and the field (`{{work_unit.findings}}`),
//! the same in any format of text; a list of an evaluation is a block whose
//! text is written once for each of its elements.
//!
//! ```text
//! a field of a model ──► marker("work_unit", "findings") ──► "{{work_unit.findings}}"
//!
//! {{#evaluation_plan.work_units}} ... {{work_unit.label}} ... {{/evaluation_plan.work_units}}
//! {{#evaluation_plan.work_units verdict=fail}} ... {{/evaluation_plan.work_units}}   only those failed
//!
//! a template ──► segments ──► [Text, Block { list, filter, body }, Text, ...]  (a block never in a block)
//! a text, the values of its markers ──► replaced ──► each known marker replaced, in one pass
//! a template ──► template_warnings ──► ["an unknown marker {{work_unit.findngs}}"]
//! ```
//!
//! A marker the program does not know stays as it is written, and the
//! editor warns of it; a block not closed or in a block is refused.

use crate::objects::error::DomainError;
use crate::objects::evaluation::evaluation_plan::evaluation_plan_rules::fields::EVALUATION_PLAN_FIELDS;
use crate::objects::evaluation::lab_proof::lab_proof_rules::LAB_PROOF_FIELDS;
use crate::objects::evaluation::oscal_standard_document::oscal_standard_document_rules::OSCAL_STANDARD_DOCUMENT_FIELDS;
use crate::objects::evaluation::product_document::product_document_rules::PRODUCT_DOCUMENT_FIELDS;
use crate::objects::evaluation::work_unit::work_unit_model::Verdict;
use crate::objects::evaluation::work_unit::work_unit_rules::fields::WORK_UNIT_FIELDS;

/// The model of the evaluation, whose markers are known in the whole template.
pub const EVALUATION_PLAN: &str = "evaluation_plan";

/// The most warnings of a template given to the editor.
const MAX_WARNINGS: usize = 20;

/// A list of an evaluation a block is written for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum List {
    /// The standards of the evaluation (`EvaluationPlan::oscal_documents`).
    OscalDocuments,
    /// The documents of the product (`EvaluationPlan::product_documents`).
    ProductDocuments,
    /// The proofs of the laboratory its work units need.
    LabProofs,
    /// The work units, in the order of the work (`EvaluationPlan::work_units`).
    WorkUnits,
}

/// Every list, in the order the editor shows them.
pub const LISTS: [List; 4] = [List::OscalDocuments, List::ProductDocuments, List::LabProofs, List::WorkUnits];

/// Which elements of a list a block is written for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    /// Every element.
    All,
    /// Only the work units of this verdict; `None`: not assessed.
    Verdict(Option<Verdict>),
}

/// A part of a template: a text, or a block written for a list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Segment<'a> {
    /// A text outside every block.
    Text(&'a str),
    /// A block: its list, its filter and its text between its tags.
    Block { list: List, filter: Filter, body: &'a str },
}

/// The marker of the field `field` of the model `model`.
///
/// Input: `model`, the name of a model (`"work_unit"`); `field`, one of its
/// fields (`"findings"`).
/// Output: the marker (`"{{work_unit.findings}}"`).
///
/// Called by: [`element_markers`], [`known_markers`];
/// `objects/evaluation/report_template/report_template_rules/filling.rs`: `values_of`.
pub fn marker(model: &str, field: &str) -> String {
    format!("{{{{{model}.{field}}}}}")
}

/// The name of the list `list` in the tags of its block (`"evaluation_plan.work_units"`).
///
/// Called by: [`list_named`], [`block_tags`].
fn list_name(list: List) -> &'static str {
    match list {
        List::OscalDocuments => "evaluation_plan.oscal_documents",
        List::ProductDocuments => "evaluation_plan.product_documents",
        List::LabProofs => "evaluation_plan.lab_proofs",
        List::WorkUnits => "evaluation_plan.work_units",
    }
}

/// The model of the elements of the list `list` and its fields
/// (`("work_unit", WORK_UNIT_FIELDS)`).
///
/// Called by: [`element_markers`]; `objects/evaluation/report_template/report_template_rules/filling.rs`:
/// `element_of`.
pub fn element_model(list: List) -> (&'static str, &'static [&'static str]) {
    match list {
        List::OscalDocuments => ("oscal_standard_document", &OSCAL_STANDARD_DOCUMENT_FIELDS),
        List::ProductDocuments => ("product_document", &PRODUCT_DOCUMENT_FIELDS),
        List::LabProofs => ("lab_proof", &LAB_PROOF_FIELDS),
        List::WorkUnits => ("work_unit", &WORK_UNIT_FIELDS),
    }
}

/// The list named `name` in the tags of a block.
///
/// Input: `name`, the name after `{{#` or `{{/` (`"evaluation_plan.work_units"`).
/// Output: the list (`Some(List::WorkUnits)`); `None` for no list.
///
/// Called by: [`opening_of`].
/// Calls: [`list_name`].
fn list_named(name: &str) -> Option<List> {
    let mut found = None;

    for list in LISTS {
        if list_name(list) == name {
            found = Some(list);
        }
    }

    found
}

/// The tags that open and close the block of the list `list`
/// (`("{{#evaluation_plan.work_units}}", "{{/evaluation_plan.work_units}}")`).
///
/// Called by: `panels/views/reports/templates.rs`: `report_markers`.
/// Calls: [`list_name`].
pub fn block_tags(list: List) -> (String, String) {
    let name = list_name(list);

    (format!("{{{{#{name}}}}}"), format!("{{{{/{name}}}}}"))
}

/// The markers of the fields of the elements of the list `list`, known
/// only inside its block.
///
/// Input: `list`, the list of a block.
/// Output: the markers (`["{{work_unit.id}}", ..., "{{work_unit.author}}"]`).
///
/// Called by: [`known_markers`]; `panels/views/reports/templates.rs`: `report_markers`.
/// Calls: [`element_model`], [`marker`].
pub fn element_markers(list: List) -> Vec<String> {
    let (model, fields) = element_model(list);
    let mut markers = Vec::with_capacity(fields.len());

    for field in fields {
        markers.push(marker(model, field));
    }

    markers
}

/// Every marker known in the text of the list `list`: those of the
/// evaluation, and inside a block those of its elements.
///
/// Input: `list`, the list of the block; `None` outside every block.
/// Output: the markers (`["{{evaluation_plan.id}}", ..., "{{work_unit.author}}"]`).
///
/// Called by: [`template_warnings`]; `panels/views/reports/templates.rs`: `report_markers`.
/// Calls: [`marker`], [`element_markers`].
pub fn known_markers(list: Option<List>) -> Vec<String> {
    let mut known = Vec::with_capacity(EVALUATION_PLAN_FIELDS.len() + WORK_UNIT_FIELDS.len());

    for field in EVALUATION_PLAN_FIELDS {
        known.push(marker(EVALUATION_PLAN, field));
    }

    if let Some(list) = list {
        known.extend(element_markers(list));
    }

    known
}

/// The filter of a block written after its list (`verdict=fail`).
///
/// Input: `list`, the list of the block; `filter`, the text after its name,
/// trimmed ("verdict=fail", "" for none).
/// Output: the filter (`Filter::Verdict(Some(Verdict::Fail))`);
/// `DomainError::Refused` for another text, or a filter of a list of no
/// work units.
///
/// Called by: [`opening_of`].
fn filter_of(list: List, filter: &str) -> Result<Filter, DomainError> {
    if filter.is_empty() {
        return Ok(Filter::All);
    }

    ensure!(list == List::WorkUnits, DomainError::Refused(format!("only work units take a filter: {filter}")));

    match filter {
        "verdict=pass" => Ok(Filter::Verdict(Some(Verdict::Pass))),
        "verdict=fail" => Ok(Filter::Verdict(Some(Verdict::Fail))),
        "verdict=inconclusive" => Ok(Filter::Verdict(Some(Verdict::Inconclusive))),
        "verdict=not_applicable" => Ok(Filter::Verdict(Some(Verdict::NotApplicable))),
        "verdict=none" => Ok(Filter::Verdict(None)),
        _ => Err(DomainError::Refused(format!(
            "an unknown filter {filter}: verdict=pass, fail, inconclusive, not_applicable or none"
        ))),
    }
}

/// The list and the filter of the tag that opens a block, without its braces.
///
/// Input: `tag`, the text between `{{#` and `}}` (`"evaluation_plan.work_units verdict=fail"`).
/// Output: the list and the filter; `DomainError::Refused` for no list or an
/// unknown filter.
///
/// Called by: [`segments`].
/// Calls: [`list_named`], [`filter_of`].
fn opening_of(tag: &str) -> Result<(List, Filter), DomainError> {
    let (name, filter) = tag.trim().split_once(' ').unwrap_or((tag.trim(), ""));
    let Some(list) = list_named(name) else {
        return Err(DomainError::Refused(format!("a block of an unknown list: {{{{#{tag}}}}}")));
    };

    Ok((list, filter_of(list, filter.trim())?))
}

/// The template `text` parted into its texts and its blocks.
///
/// Input: `text`, a template, bounded when it was written (`templates.rs`).
/// Output: the parts in their order (`[Text("# R7\n"), Block { list: WorkUnits, .. }]`);
/// `DomainError::Refused` for a block not closed, a block in a block, a tag
/// that closes no block, or the tag of an unknown list or filter.
///
/// Called by: [`template_warnings`]; `objects/evaluation/report_template/report_template_rules/filling.rs`:
/// `filled_report`.
/// Calls: [`opening_of`], [`list_name`].
pub fn segments(text: &str) -> Result<Vec<Segment<'_>>, DomainError> {
    let mut parts = Vec::new();
    let mut rest = text;

    // Each pass takes at least one block, which is longer than one byte, so
    // it ends within the length of the template.
    for _ in 0..=text.len() {
        let Some((before, after)) = rest.split_once("{{#") else {
            break;
        };

        ensure!(!before.contains("{{/"), DomainError::Refused("a tag {{/...}} closes no block".to_owned()));

        let Some((tag, after_tag)) = after.split_once("}}") else {
            return Err(DomainError::Refused("a tag {{#... without its }}".to_owned()));
        };
        let (list, filter) = opening_of(tag)?;
        let closing = format!("{{{{/{}}}}}", list_name(list));
        let Some((body, after_block)) = after_tag.split_once(closing.as_str()) else {
            return Err(DomainError::Refused(format!("the block {{{{#{tag}}}}} is not closed with {closing}")));
        };

        ensure!(
            !body.contains("{{#") && !body.contains("{{/"),
            DomainError::Refused(format!("a block in the block {{{{#{tag}}}}}: blocks are never nested"))
        );

        parts.push(Segment::Text(before));
        parts.push(Segment::Block { list, filter, body });
        rest = after_block;
    }

    ensure!(
        !rest.contains("{{#") && !rest.contains("{{/"),
        DomainError::Refused("a tag {{/...}} closes no block".to_owned())
    );
    parts.push(Segment::Text(rest));
    Ok(parts)
}

/// The value of the marker that `text` starts with, and the text after it.
///
/// Input: `text`, the rest of a text from a `{{` on; `values`, the markers
/// with their values.
/// Output: the value and the rest (`Some(("Pass", " ..."))`); `None` when
/// no marker of `values` starts there.
///
/// Called by: [`replaced`].
fn value_at<'a, 'b>(text: &'b str, values: &'a [(String, String)]) -> Option<(&'a str, &'b str)> {
    for (marker, value) in values {
        if let Some(rest) = text.strip_prefix(marker.as_str()) {
            return Some((value, rest));
        }
    }

    None
}

/// The text `text` with each marker of `values` replaced by its value, in
/// one pass: a marker inside a value (findings that write
/// `{{work_unit.label}}`) is never replaced again, and a marker the program
/// does not know stays as it is written.
///
/// Input: `text`, a part of a template; `values`, the markers with their
/// values (`[("{{work_unit.verdict}}", "Pass")]`).
/// Output: the text ("Verdict: Pass").
///
/// Called by: `objects/evaluation/report_template/report_template_rules/filling.rs`: `report_text`.
/// Calls: [`value_at`].
pub fn replaced(text: &str, values: &[(String, String)]) -> String {
    let mut written = String::with_capacity(text.len());
    let mut rest = text;

    // Each pass takes at least two bytes of the text, so it ends within its
    // length.
    for _ in 0..=text.len() {
        let Some(start) = rest.find("{{") else {
            break;
        };
        let (before, from_marker) = rest.split_at(start);

        written.push_str(before);

        if let Some((value, after)) = value_at(from_marker, values) {
            written.push_str(value);
            rest = after;
        } else {
            written.push_str("{{");
            rest = &from_marker[2..];
        }
    }

    written.push_str(rest);
    written
}

/// The markers of the text `text` that are not among `known`, each once.
///
/// Input: `text`, a part of a template; `known`, the markers known there
/// ([`known_markers`]); `found`, the unknown ones found so far, added to.
/// Output: nothing; `found` holds at most [`MAX_WARNINGS`].
///
/// Called by: [`template_warnings`].
fn add_unknown(text: &str, known: &[String], found: &mut Vec<String>) {
    let mut rest = text;

    // Each pass takes at least two bytes of the text.
    for _ in 0..=text.len() {
        let Some((_, after)) = rest.split_once("{{") else {
            break;
        };
        let Some((name, after_marker)) = after.split_once("}}") else {
            break;
        };
        let marker = format!("{{{{{name}}}}}");

        if !known.contains(&marker) && !found.contains(&marker) && found.len() < MAX_WARNINGS {
            found.push(marker);
        }

        rest = after_marker;
    }
}

/// The warnings of the template `text`: each marker the program does not
/// know where it stands (a field misspelled, or a field of a work unit
/// outside its block), which a report writes as it is.
///
/// Input: `text`, a template, bounded when it was written.
/// Output: the warnings, at most [`MAX_WARNINGS`] (`["an unknown marker
/// {{work_unit.findngs}}"]`); an error of [`segments`] for a block not
/// closed or nested.
///
/// Called by: `objects/evaluation/report_template/report_template_rules/templates.rs`: `change_template`;
/// `panels/views/reports/templates.rs`: `report_template_get`.
/// Calls: [`segments`], [`known_markers`], [`add_unknown`].
pub fn template_warnings(text: &str) -> Result<Vec<String>, DomainError> {
    let mut unknown = Vec::new();

    for part in segments(text)? {
        match part {
            Segment::Text(text) => add_unknown(text, &known_markers(None), &mut unknown),
            Segment::Block { list, body, .. } => add_unknown(body, &known_markers(Some(list)), &mut unknown),
        }
    }

    let mut warnings = Vec::with_capacity(unknown.len());

    for marker in unknown {
        warnings.push(format!("an unknown marker {marker}: it is written as it stands"));
    }

    Ok(warnings)
}

#[cfg(test)]
mod tests {
    //! Tests of the markers: named by the model and the field, blocks parted
    //! and refused when wrong, replaced in one pass, unknown ones warned of.

    use super::*;

    #[test]
    fn a_field_is_a_marker_and_a_marker_is_replaced_once() {
        assert_eq!(marker("work_unit", "findings"), "{{work_unit.findings}}");

        let values = vec![
            ("{{work_unit.label}}".to_owned(), "ASE_INT.1-1".to_owned()),
            ("{{work_unit.findings}}".to_owned(), "it writes {{work_unit.label}}".to_owned()),
        ];

        assert_eq!(
            replaced("{{work_unit.label}}: {{work_unit.findings}} {{x}} {{", &values),
            "ASE_INT.1-1: it writes {{work_unit.label}} {{x}} {{"
        );
    }

    #[test]
    fn a_template_is_parted_into_its_texts_and_blocks() {
        let parts = segments(
            "# {{evaluation_plan.product}}\n{{#evaluation_plan.work_units verdict=fail}}- {{work_unit.label}}\n\
             {{/evaluation_plan.work_units}}end",
        )
        .unwrap();

        assert_eq!(
            parts,
            vec![
                Segment::Text("# {{evaluation_plan.product}}\n"),
                Segment::Block {
                    list: List::WorkUnits,
                    filter: Filter::Verdict(Some(Verdict::Fail)),
                    body: "- {{work_unit.label}}\n"
                },
                Segment::Text("end"),
            ]
        );
    }

    #[test]
    fn a_block_not_closed_nested_or_unknown_is_refused() {
        let wrong = [
            "{{#evaluation_plan.work_units}} no end",
            "{{#evaluation_plan.work_units}}{{#evaluation_plan.lab_proofs}}{{/evaluation_plan.lab_proofs}}\
             {{/evaluation_plan.work_units}}",
            "{{/evaluation_plan.work_units}}",
            "{{#evaluation_plan.chats}}{{/evaluation_plan.chats}}",
            "{{#evaluation_plan.lab_proofs verdict=fail}}{{/evaluation_plan.lab_proofs}}",
            "{{#evaluation_plan.work_units verdict=maybe}}{{/evaluation_plan.work_units}}",
        ];

        for template in wrong {
            assert!(matches!(segments(template), Err(DomainError::Refused(_))), "{template}");
        }
    }

    #[test]
    fn an_unknown_marker_or_one_outside_its_block_is_warned_of() {
        let warnings = template_warnings(
            "{{work_unit.label}} {{evaluation_plan.product}}\n\
             {{#evaluation_plan.work_units}}{{work_unit.findngs}} {{work_unit.label}}{{/evaluation_plan.work_units}}",
        )
        .unwrap();

        assert_eq!(warnings.len(), 2);
        assert!(warnings[0].contains("{{work_unit.label}}"));
        assert!(warnings[1].contains("{{work_unit.findngs}}"));
    }
}
