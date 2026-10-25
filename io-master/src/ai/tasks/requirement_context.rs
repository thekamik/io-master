//! The whole requirement for the works of the AI of the Evaluation lab
//! (step 57.1 of docs/todo.md): where it stands in its catalog, its parts
//! word for word, its parameters (the assignments and selections of a
//! standard), the requirements it links to, and the text of its related
//! elements (for example the work units of the methodology). On plain data;
//! the view gathers it from the catalogs (`views/evaluation_lab/lab_task/read.rs`).

use crate::model::oscal::catalog::Control;
use crate::model::oscal::common::Parameter;

use super::candidates::requirement_text;

/// The most characters of the related elements in one question; the
/// elements past it are counted, not given.
pub const RELATED_CHARS: usize = 24_000;

/// A requirement that a control links to (a dependency of a component of
/// Common Criteria, a related control), as the question names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Linked {
    /// The relation the catalog names (`required`, `related`, ...).
    pub relation: String,
    /// Its label and title.
    pub text: String,
}

/// The text of the requirement `control` (shown as `label`) of catalog
/// `catalog`, under the groups and controls `above` it: where it stands,
/// its parts word for word, its parameters and what it links to. Fails
/// when its parts nest deeper than a catalog may.
///
/// Called by: `views/evaluation_lab/lab_task/read.rs` (`requirement`).
/// Calls: `requirement_text`, [`parameter`].
pub fn requirement(
    catalog: &str,
    above: &[String],
    label: &str,
    control: &Control,
    linked: &[Linked],
) -> Result<String, String> {
    let mut text = format!("Catalog: {catalog}\n");

    if !above.is_empty() {
        text.push_str(&format!("Within: {}\n", above.join(" > ")));
    }

    text.push_str(&requirement_text(label, control)?);

    if !control.params.is_empty() {
        text.push_str("Its parameters (the places `{{ insert: param, id }}` in the text):\n");

        for param in &control.params {
            text.push_str(&parameter(param));
        }
    }

    if !linked.is_empty() {
        text.push_str("The requirements it links to:\n");

        for link in linked {
            text.push_str(&format!("- {}: {}\n", link.relation, link.text));
        }
    }

    Ok(text)
}

/// One line of a parameter: its id, its label, the choices of a
/// selection, the values set, and its usage.
///
/// Called by: [`requirement`].
fn parameter(param: &Parameter) -> String {
    let mut line = format!("- {}", param.id);

    if let Some(label) = &param.label {
        line.push_str(&format!(": assignment of {label}"));
    }

    if let Some(select) = &param.select {
        let how = if select.how_many.as_deref() == Some("one-or-more") { "one or more" } else { "one" };
        line.push_str(&format!("; selection of {how} of: {}", select.choice.join("; ")));
    }

    if !param.values.is_empty() {
        line.push_str(&format!("; set to: {}", param.values.join("; ")));
    }

    if let Some(usage) = &param.usage {
        line.push_str(&format!(" ({usage})"));
    }

    line.push('\n');
    line
}

/// The text of the related elements `texts` (each the text of one
/// requirement) for a question, up to [`RELATED_CHARS`]; the elements past
/// it are counted. Empty when there is none.
///
/// Called by: `views/evaluation_lab/lab_task/read.rs` (`related_text`).
pub fn related(texts: &[String]) -> String {
    let mut text = String::new();
    let mut left_out = 0;

    for element in texts {
        if text.len() + element.len() > RELATED_CHARS {
            left_out += 1;
            continue;
        }

        text.push_str(element);
    }

    if left_out > 0 {
        text.push_str(&format!("({left_out} more related elements left out: the question would be too long)\n"));
    }

    text
}

#[cfg(test)]
mod tests {
    //! Unit tests of `requirement_context.rs`.

    use super::*;
    use crate::model::oscal::common::{Other, Selection};

    fn control(params: Vec<Parameter>) -> Control {
        serde_json::from_value::<Control>(serde_json::json!({
            "id": "fcs_ckm.1", "title": "Cryptographic key generation",
            "parts": [{"name": "statement", "prose": "The TSF shall generate keys of {{ insert: param, p1 }}."}]
        }))
        .map(|mut control| {
            control.params = params;
            control
        })
        .unwrap()
    }

    fn param(label: Option<&str>, select: Option<Selection>) -> Parameter {
        Parameter {
            id: "p1".to_owned(),
            class: None,
            props: Vec::new(),
            links: Vec::new(),
            label: label.map(str::to_owned),
            usage: None,
            values: Vec::new(),
            select,
            other: Other::default(),
        }
    }

    #[test]
    fn the_requirement_says_where_it_stands_and_what_it_links_to() {
        let linked =
            [Linked { relation: "required".to_owned(), text: "FCS_COP.1: Cryptographic operation".to_owned() }];
        let above = ["Class FCS: Cryptographic support".to_owned(), "FCS_CKM".to_owned()];

        let text = requirement("CC Part 2", &above, "FCS_CKM.1", &control(Vec::new()), &linked).unwrap();

        assert!(text.starts_with("Catalog: CC Part 2\nWithin: Class FCS: Cryptographic support > FCS_CKM\n"));
        assert!(text.contains("The TSF shall generate keys"));
        assert!(text.contains("- required: FCS_COP.1: Cryptographic operation"));
        assert!(!text.contains("parameters"));
    }

    #[test]
    fn the_parameters_are_explained() {
        let select =
            Selection { how_many: Some("one-or-more".to_owned()), choice: vec!["RSA".to_owned(), "ECC".to_owned()] };

        let text =
            requirement("CC", &[], "FCS_CKM.1", &control(vec![param(Some("key sizes"), Some(select))]), &[]).unwrap();

        assert!(text.contains("- p1: assignment of key sizes; selection of one or more of: RSA; ECC\n"));
        assert!(!text.contains("Within"));
    }

    #[test]
    fn the_related_elements_stop_at_their_limit() {
        let long = "w".repeat(RELATED_CHARS - 10);

        let text = related(&["AVA_VAN.1-1 first\n".to_owned(), long.clone(), long]);

        assert!(text.starts_with("AVA_VAN.1-1 first\n"));
        assert!(text.ends_with("(2 more related elements left out: the question would be too long)\n"));
        assert!(related(&[]).is_empty());
    }
}
