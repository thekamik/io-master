//! A requirement of the laboratory's own, outside the standards (step 51.1e
//! of docs/history.md): its identifier, written by the planner as they choose
//! (the program imposes no naming), its title and its statement. It is kept
//! as a control in one shared catalog of OSCAL, "Requirements of the
//! laboratory", so a task points to it as to a requirement of a standard.

use super::oscal::catalog::Control;
use super::oscal::common::{Other, Part, Property};
use super::oscal::values::token;
use super::text::{Field, Invalid, check_text};

/// The title of the shared catalog of the requirements of the laboratory.
pub const CATALOG_TITLE: &str = "Requirements of the laboratory";

/// The longest identifier.
pub const IDENTIFIER_MAX: usize = 60;

/// The longest title.
pub const TITLE_MAX: usize = 200;

/// The longest statement.
pub const STATEMENT_MAX: usize = 20_000;

/// A requirement of the laboratory, as the planner writes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnRequirement {
    pub identifier: String,
    pub title: String,
    pub statement: String,
}

/// The fields in their normal form: no spaces around them.
///
/// Called by: the commands of the requirements of the laboratory
/// (`views/evaluation_plan/evaluation_plan.rs`), before [`check`].
pub fn normalized(requirement: &OwnRequirement) -> OwnRequirement {
    OwnRequirement {
        identifier: requirement.identifier.trim().to_owned(),
        title: requirement.title.trim().to_owned(),
        statement: requirement.statement.trim().to_owned(),
    }
}

/// Checks a requirement: an identifier and a title of one line, a
/// statement; none of them empty or too long.
///
/// Called by: the commands of the requirements of the laboratory.
pub fn check(requirement: &OwnRequirement) -> Result<(), Invalid> {
    let mut has_letter = false;

    for character in requirement.identifier.chars() {
        if character.is_alphanumeric() {
            has_letter = true;
        }
    }

    if !has_letter {
        return Err(Invalid::new(Field::Identifier, "the requirement needs an identifier with a letter or a digit"));
    }

    if requirement.title.is_empty() {
        return Err(Invalid::new(Field::Title, "the requirement needs a title"));
    }

    if requirement.statement.is_empty() {
        return Err(Invalid::new(Field::Text, "the requirement needs its statement"));
    }

    check_text(Field::Identifier, &requirement.identifier, IDENTIFIER_MAX, false)?;
    check_text(Field::Title, &requirement.title, TITLE_MAX, false)?;
    check_text(Field::Text, &requirement.statement, STATEMENT_MAX, true)?;

    Ok(())
}

/// The id of the control of a requirement: its identifier made a token of
/// OSCAL (`Net config 1` becomes `Net_config_1`).
///
/// Called by: [`control`], and the commands that look the control up.
pub fn control_id(requirement: &OwnRequirement) -> String {
    token(&requirement.identifier)
}

/// The control of OSCAL that keeps `requirement`: the identifier as its
/// label, the statement as its part `statement`.
///
/// Called by: the commands that add or change a requirement of the laboratory.
pub fn control(requirement: &OwnRequirement) -> Control {
    let id = control_id(requirement);

    Control {
        id: id.clone(),
        class: None,
        title: requirement.title.clone(),
        params: Vec::new(),
        props: vec![Property {
            name: "label".to_owned(),
            value: requirement.identifier.clone(),
            ns: None,
            class: None,
            other: Other::default(),
        }],
        links: Vec::new(),
        parts: vec![Part {
            id: Some(format!("{id}_smt")),
            name: "statement".to_owned(),
            ns: None,
            class: None,
            title: None,
            props: Vec::new(),
            prose: Some(requirement.statement.clone()),
            parts: Vec::new(),
            links: Vec::new(),
            other: Other::default(),
        }],
        controls: Vec::new(),
        other: Other::default(),
    }
}

/// The requirement a control of the shared catalog keeps, to be changed by
/// the planner: its label (else its id), its title, the prose of its part
/// `statement`.
///
/// Called by: `standard_browser/requirement.rs`, for a requirement of the
/// laboratory.
pub fn of_control(control: &Control) -> OwnRequirement {
    let mut statement = String::new();

    for part in &control.parts {
        if part.name == "statement"
            && let Some(prose) = &part.prose
        {
            statement = prose.clone();
        }
    }

    OwnRequirement {
        identifier: control.label().unwrap_or(&control.id).to_owned(),
        title: control.title.clone(),
        statement,
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests of `own_requirement.rs`.

    use super::*;

    fn requirement(identifier: &str, title: &str, statement: &str) -> OwnRequirement {
        OwnRequirement { identifier: identifier.to_owned(), title: title.to_owned(), statement: statement.to_owned() }
    }

    #[test]
    fn every_field_is_needed_and_of_its_length() {
        assert!(check(&requirement("Net config 1", "Network configuration", "Review the configuration.")).is_ok());
        assert!(check(&requirement("", "t", "s")).is_err());
        assert!(check(&requirement("!!!", "t", "s")).is_err(), "no letter or digit for an id");
        assert!(check(&requirement("x", "", "s")).is_err());
        assert!(check(&requirement("x", "t", "")).is_err());
        assert!(check(&requirement("x", "two\nlines", "s")).is_err());
    }

    #[test]
    fn a_requirement_becomes_a_control_and_back() {
        let written = requirement("Net config 1", "Network configuration", "Review it.\nAll of it.");

        let made = control(&written);

        assert_eq!(made.id, control_id(&written));
        assert_eq!(made.label(), Some("Net config 1"));
        assert_eq!(of_control(&made), written);
    }

    #[test]
    fn the_normal_form_has_no_spaces_around() {
        assert_eq!(normalized(&requirement(" A ", " t ", " s ")), requirement("A", "t", "s"));
    }
}
