//! The rules every text field of the program follows, the error when
//! one is broken, and the names of the fields and of the objects the
//! errors of the rules speak of (as enums, so that the window can tell
//! them apart). The areas (`standards.rs`, and the documents of OSCAL of step 41,
//! `standards.rs`) check their fields with [`check_text`] and turn
//! [`Invalid`] into their own errors.

use std::fmt;

/// A field of a project, a scheme, a tile, an evaluation or a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Field {
    Description,
    Documents,
    Evaluator,
    Glossary,
    Identifier,
    Instructions,
    Kind,
    Name,
    Part,
    Place,
    Position,
    Product,
    ProductVersion,
    Project,
    Quote,
    Text,
    Title,
    Version,
}

impl Field {
    /// The name of the field, in the sentences of the errors.
    pub fn name(self) -> &'static str {
        match self {
            Field::Description => "description",
            Field::Documents => "documents",
            Field::Evaluator => "evaluator",
            Field::Glossary => "glossary",
            Field::Identifier => "identifier",
            Field::Instructions => "instructions",
            Field::Kind => "kind",
            Field::Name => "name",
            Field::Part => "part",
            Field::Place => "place",
            Field::Position => "position",
            Field::Product => "product",
            Field::ProductVersion => "product version",
            Field::Project => "project",
            Field::Quote => "quote",
            Field::Text => "text",
            Field::Title => "title",
            Field::Version => "version",
        }
    }
}

impl fmt::Display for Field {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

/// A field that breaks a rule: which field, and why (a sentence for
/// the evaluator).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invalid {
    pub field: Field,
    pub reason: String,
}

impl Invalid {
    pub fn new(field: Field, reason: impl Into<String>) -> Invalid {
        Invalid { field, reason: reason.into() }
    }
}

impl fmt::Display for Invalid {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.reason)
    }
}

impl std::error::Error for Invalid {}

/// A text field: at most `max` characters; no control characters
/// other than line breaks and tabs where `lines` allows them.
pub fn check_text(field: Field, text: &str, max: usize, lines: bool) -> Result<(), Invalid> {
    if text.chars().count() > max {
        return Err(Invalid::new(field, format!("the {field} is longer than {max} characters")));
    }

    let bad = |character: char| character.is_control() && !(lines && (character == '\n' || character == '\t'));

    if text.chars().any(bad) {
        return Err(Invalid::new(field, format!("the {field} holds a control character")));
    }

    Ok(())
}

/// A name for a file proposed from the title `title` of what is exported:
/// letters, digits, `-`, `_`, `.` and spaces kept, everything else `_`; at
/// most 100 characters; `fallback` when nothing is left.
///
/// Called by: the exports of the views (`oscal_documents_export`, `work_packages_export`, `agent_form_export`).
pub fn safe_file_name(title: &str, fallback: &str) -> String {
    let mut name = String::new();

    for character in title.trim().chars().take(100) {
        if character.is_alphanumeric() || "-_ .".contains(character) {
            name.push(character);
        } else {
            name.push('_');
        }
    }

    if name.is_empty() { fallback.to_owned() } else { name }
}

#[cfg(test)]
mod tests {
    //! Unit tests of `text.rs`.

    use super::*;
    use crate::model::text::Field;

    #[test]
    fn a_text_keeps_its_length_and_has_no_control_characters() {
        assert!(check_text(Field::Name, "Anna", 4, false).is_ok());
        assert_eq!(
            check_text(Field::Name, "Annas", 4, false),
            Err(Invalid::new(Field::Name, "the name is longer than 4 characters"))
        );
        assert_eq!(
            check_text(Field::Name, "a\tb", 10, false).unwrap_err().reason,
            "the name holds a control character"
        );
        assert!(check_text(Field::Description, "a\nb\tc", 10, true).is_ok());
        assert!(check_text(Field::Description, "a\u{7}b", 10, true).is_err());
    }

    #[test]
    fn a_file_name_keeps_only_safe_characters() {
        assert_eq!(safe_file_name(" CC Part 2: SFR / v1.0 ", "oscal"), "CC Part 2_ SFR _ v1.0");
        assert_eq!(safe_file_name("../../etc", "oscal"), ".._.._etc");
        assert_eq!(safe_file_name("   ", "oscal"), "oscal");
    }
}
