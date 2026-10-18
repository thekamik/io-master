//! Unit tests of `catalog_section.rs`, on data in memory; the controls
//! read are checked against the schema of NIST inside a catalog.

use super::*;

use crate::model::oscal::OscalDocument;

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
