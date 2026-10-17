//! The rules every text field of the program follows, and the error
//! when one is broken. The areas (`test_projects.rs`, `evaluations.rs`,
//! `standards.rs`) check their fields with [`check_text`] and turn
//! [`Invalid`] into their own errors.

use std::fmt;

/// A field that breaks a rule: which field, and why (a sentence for
/// the evaluator).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invalid {
    pub field: &'static str,
    pub reason: String,
}

impl Invalid {
    pub fn new(field: &'static str, reason: impl Into<String>) -> Invalid {
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
pub fn check_text(field: &'static str, text: &str, max: usize, lines: bool) -> Result<(), Invalid> {
    if text.chars().count() > max {
        return Err(Invalid::new(field, format!("the {field} is longer than {max} characters")));
    }

    let bad = |character: char| character.is_control() && !(lines && (character == '\n' || character == '\t'));

    if text.chars().any(bad) {
        return Err(Invalid::new(field, format!("the {field} holds a control character")));
    }

    Ok(())
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "text_tests.rs"]
mod tests;
