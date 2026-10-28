//! The task `catalog_section` of the AI (role *text analysis*): the
//! question with the pages of one group of a document of a standard,
//! and the reading of the answer: the controls of an OSCAL catalog, each
//! with its statement and its items, its parameters (the operations
//! "assignment" and "selection" of a standard), its guidance, and its
//! assessment objectives with their methods (examine, interview, test),
//! proposed with the passages they come from and a note of the AI.
//! Applying them is the work of the business; the evaluator accepts or
//! rejects every one.
//!
//! The program knows no standard: what a control is in a given standard
//! (a component of Common Criteria, a work unit of the CEM, a provision
//! of ETSI EN 303 645, a test case) is said by the instructions, which
//! the evaluator may change.

use serde_json::Value;

use crate::model_old::oscal::catalog::Control;
use crate::model_old::oscal::common::{Parameter, Part, Property, Selection};
use crate::model_old::oscal::values::token;
use crate::model_old::standards::{Document, Part as PagePart};

use super::catalog_outline::{document_name, label, quote_of};
use super::json::{json_object, text_of};

/// The instructions when the evaluator has not changed them.
pub const DEFAULT_INSTRUCTIONS: &str = r#"You help to represent a document of a standard in NIST OSCAL, as a catalog. Read the pages below, which belong to one group of the catalog, and propose its requirements as controls. A control is, depending on the document: a component of Common Criteria (FAU_GEN.1, ADV_FSP.4), a work unit of the CEM (ADV_FSP.4-1), a provision of ETSI EN 303 645 (5.1-1), a test case of a test specification. Rules:
1. Copy the identifier and the title of each control exactly as the document gives them.
2. "statement": the text of the requirement, as items when it has several elements (for Common Criteria: the elements such as ADV_FSP.4.1D, ADV_FSP.4.1C, ADV_FSP.4.1E), each with its identifier and its exact text.
3. "params": every place the requirement leaves open: an assignment (a value to give: "label") or a selection (a choice: "choices"). In the text of the statement write such a place as {{ insert: param, <id> }}.
4. "objectives": what the evaluator must establish to assess the control (for the CEM: the work units; for a test specification: the test steps), each with its identifier, its text, and its method: EXAMINE, INTERVIEW or TEST.
5. "guidance": application notes, if the document gives them.
6. Quote the exact text of the document each control is based on, with its page number (copy it, do not paraphrase). A quote that is not in the text will be marked as not found.
7. "note": your short explanation (for example what you were not sure of). It is shown as a note of the AI, never as a quote.

Answer with one JSON object and nothing else, in this form:
{"controls": [{"identifier": "ADV_FSP.4", "title": "Complete functional specification",
  "statement": [{"identifier": "ADV_FSP.4.1D", "text": "The developer shall provide a functional specification."}],
  "params": [{"identifier": "p1", "label": "list of interfaces", "choices": []}],
  "objectives": [{"identifier": "ADV_FSP.4-1", "text": "...", "method": "EXAMINE"}],
  "guidance": "", "quotes": [{"page": 31, "text": "..."}], "note": ""}]}"#;

/// A control as the AI proposed it.
#[derive(Debug, Clone, PartialEq)]
pub struct ProposedControl {
    /// The control of OSCAL, its ids made from the identifiers of the
    /// document, the identifiers kept as labels.
    pub control: Control,
    /// The passages it comes from: page and text, not checked yet.
    pub quotes: Vec<(u32, String)>,
    /// The note of the AI.
    pub note: String,
}

/// The answer, read.
#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    pub controls: Vec<ProposedControl>,
    pub dropped: Vec<String>,
}

/// The message to the model: the instructions, the group, the controls
/// the catalog has already, and the text of `part`.
pub fn prompt(
    instructions: &str,
    document: &Document,
    part: &PagePart,
    group: Option<&str>,
    known: &[(String, String)],
) -> String {
    let mut text = format!("{}\n\n", instructions.trim());

    if let Some(group) = group {
        text.push_str(&format!("The group read: {group}.\n\n"));
    }

    if !known.is_empty() {
        text.push_str("Controls the catalog has already (do not repeat them):\n");

        for (id, title) in known {
            text.push_str(&format!("- {id}: {title}\n"));
        }

        text.push('\n');
    }

    text.push_str(&format!(
        "The document: {}, pages {} to {}.\n\n{}",
        document_name(document),
        part.first_page,
        part.last_page,
        part.text
    ));

    text
}

/// A part of OSCAL with a name, an id, a label and a prose.
fn part(name: &str, id: String, identifier: &str, prose: String, props: Vec<Property>) -> Part {
    let mut all = if identifier.is_empty() { Vec::new() } else { vec![label(identifier)] };
    all.extend(props);

    Part {
        id: Some(id),
        name: name.to_owned(),
        ns: None,
        class: None,
        title: None,
        props: all,
        prose: Some(prose),
        parts: Vec::new(),
        links: Vec::new(),
        other: Default::default(),
    }
}

/// The parts of a control from the items of the answer.
fn parts_of(id: &str, item: &Value) -> Vec<Part> {
    let mut parts = Vec::new();
    let items: Vec<Part> = item
        .get("statement")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(index, element)| {
            let text = text_of(element, "text");
            let identifier = text_of(element, "identifier");

            (!text.is_empty())
                .then(|| part("item", token(&format!("{id}_smt.{}", index + 1)), &identifier, text, Vec::new()))
        })
        .collect();

    if !items.is_empty() {
        let mut statement = part("statement", token(&format!("{id}_smt")), "", String::new(), Vec::new());
        statement.prose = None;
        statement.parts = items;
        parts.push(statement);
    }

    let guidance = text_of(item, "guidance");

    if !guidance.is_empty() {
        parts.push(part("guidance", token(&format!("{id}_gdn")), "", guidance, Vec::new()));
    }

    for (index, objective) in item.get("objectives").and_then(Value::as_array).into_iter().flatten().enumerate() {
        let text = text_of(objective, "text");

        if text.is_empty() {
            continue;
        }

        let method = text_of(objective, "method").to_uppercase();
        let mut objective_part = part(
            "assessment-objective",
            token(&format!("{id}_obj.{}", index + 1)),
            &text_of(objective, "identifier"),
            text,
            Vec::new(),
        );

        if ["EXAMINE", "INTERVIEW", "TEST"].contains(&method.as_str()) {
            let mut method_part = part(
                "assessment-method",
                token(&format!("{id}_obj.{}_mth", index + 1)),
                "",
                String::new(),
                vec![Property {
                    name: "method".to_owned(),
                    value: method,
                    ns: None,
                    class: None,
                    other: Default::default(),
                }],
            );
            method_part.prose = None;
            objective_part.parts.push(method_part);
        }

        parts.push(objective_part);
    }

    parts
}

/// The parameters of a control from the answer.
fn params_of(id: &str, item: &Value) -> Vec<Parameter> {
    item.get("params")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
        .map(|(index, param)| {
            let identifier = text_of(param, "identifier");
            let choices: Vec<String> = param
                .get("choices")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|choice| choice.as_str().map(|choice| choice.trim().to_owned()))
                .filter(|choice| !choice.is_empty())
                .collect();
            let label = Some(text_of(param, "label")).filter(|label| !label.is_empty());

            Parameter {
                id: token(&format!(
                    "{id}_prm_{}",
                    if identifier.is_empty() { (index + 1).to_string() } else { identifier }
                )),
                class: None,
                props: Vec::new(),
                links: Vec::new(),
                label: if choices.is_empty() { label.or(Some("a value".to_owned())) } else { label },
                usage: None,
                values: Vec::new(),
                select: (!choices.is_empty()).then_some(Selection { how_many: None, choice: choices }),
                other: Default::default(),
            }
        })
        .collect()
}

/// Writes the ids of the program in the places of the parameters the
/// model named by its own identifiers (`{{ insert: param, p1 }}`).
fn rename_params(parts: &mut [Part], renamed: &[(String, String)]) {
    for part in parts {
        if let Some(prose) = &mut part.prose {
            for (from, to) in renamed {
                *prose = prose
                    .replace(&format!("{{{{ insert: param, {from} }}}}"), &format!("{{{{ insert: param, {to} }}}}"));
            }
        }

        rename_params(&mut part.parts, renamed);
    }
}

/// Reads the answer of the model for `part`. A control that cannot be
/// read is dropped with the reason; the rest is kept in its order.
pub fn read(answer: &str, page_part: &PagePart) -> Result<Section, String> {
    let value = json_object(answer).ok_or("the answer holds no JSON object")?;
    let mut section = Section { controls: Vec::new(), dropped: Vec::new() };

    for (index, item) in value.get("controls").and_then(Value::as_array).into_iter().flatten().enumerate() {
        let identifier = text_of(item, "identifier");
        let title = text_of(item, "title");

        if title.is_empty() || title.contains('\n') {
            section.dropped.push(format!("control {}: no title of one line", index + 1));
            continue;
        }

        let id = token(if identifier.is_empty() { &title } else { &identifier });
        let quotes: Vec<(u32, String)> = item
            .get("quotes")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|quote| quote_of(quote, page_part))
            .collect();

        let params = params_of(&id, item);
        let renamed: Vec<(String, String)> = item
            .get("params")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .zip(&params)
            .map(|(given, made)| (text_of(given, "identifier"), made.id.clone()))
            .filter(|(given, _)| !given.is_empty())
            .collect();
        let mut parts = parts_of(&id, item);
        rename_params(&mut parts, &renamed);

        section.controls.push(ProposedControl {
            control: Control {
                id: id.clone(),
                class: None,
                title,
                params,
                props: if identifier.is_empty() { Vec::new() } else { vec![label(&identifier)] },
                links: Vec::new(),
                parts,
                controls: Vec::new(),
                other: Default::default(),
            },
            quotes,
            note: text_of(item, "note"),
        });
    }

    Ok(section)
}

#[cfg(test)]
mod tests {
    //! Unit tests of `catalog_section.rs`, on data in memory; the controls
    //! read are checked against the schema of NIST inside a catalog.

    use super::*;

    use crate::model_old::oscal::OscalDocument;

    fn part() -> PagePart {
        PagePart { document_id: 3, first_page: 30, last_page: 32, text: String::new() }
    }

    const ANSWER: &str = r#"```json
{"controls": [
  {"identifier": "ADV_FSP.4", "title": "Complete functional specification",
   "statement": [{"identifier": "ADV_FSP.4.1D", "text": "The developer shall provide a functional specification."},
                 {"identifier": "ADV_FSP.4.2C", "text": "It shall describe {{ insert: param, p1 }}."}],
   "params": [{"identifier": "p1", "label": "the interfaces", "choices": []},
              {"identifier": "", "label": "", "choices": ["all", "some"]}],
   "objectives": [{"identifier": "ADV_FSP.4-1", "text": "The evaluator shall examine the specification.", "method": "examine"}],
   "guidance": "An application note.",
   "quotes": [{"page": 31, "text": "The developer shall provide"}, {"page": 99, "text": "outside"}],
   "note": "Sure."},
  {"identifier": "Y", "title": ""}
]}
```"#;

    #[test]
    fn a_control_is_read_with_its_statement_params_objectives_and_quotes() {
        let section = read(ANSWER, &part()).unwrap();
        let proposed = &section.controls[0];
        let control = &proposed.control;

        assert_eq!(control.id, "ADV_FSP.4");
        assert_eq!(control.parts[0].name, "statement");
        assert_eq!(control.parts[0].parts.len(), 2);
        assert_eq!(control.parts[0].parts[0].props[0].value, "ADV_FSP.4.1D");
        // The place of the parameter names the id of the program.
        assert_eq!(control.params[0].id, "ADV_FSP.4_prm_p1");
        assert!(control.parts[0].parts[1].prose.as_deref().unwrap().contains("{{ insert: param, ADV_FSP.4_prm_p1 }}"));
        assert_eq!(control.params[1].select.as_ref().unwrap().choice, ["all", "some"]);
        assert_eq!(control.parts[1].name, "guidance");
        assert_eq!(control.parts[2].name, "assessment-objective");
        assert_eq!(control.parts[2].parts[0].props[0].value, "EXAMINE");
        assert_eq!(proposed.quotes, [(31, "The developer shall provide".to_owned())]);
        assert_eq!(proposed.note, "Sure.");
        assert_eq!(section.dropped.len(), 1);
    }

    #[test]
    fn the_controls_read_keep_the_schema_of_nist() {
        let section = read(ANSWER, &part()).unwrap();
        let catalog = serde_json::json!({"catalog": {
            "uuid": "00000000-0000-4000-8000-000000000000",
            "metadata": {"title": "T", "last-modified": "2026-10-18T00:00:00Z", "version": "1", "oscal-version": "1.2.3"},
            "controls": [serde_json::to_value(&section.controls[0].control).unwrap()]
        }});

        OscalDocument::from_json(&catalog.to_string()).unwrap();
    }
}
