//! The data of the component "recovery code": the code shown once, after
//! the setup, a change or a recovery of the password, and after "Make a new
//! recovery code" (step 49 of docs/todo.md). The one secret that reaches the
//! page (CLAUDE.md, "State lives in the core"): the page shows it, keeps it
//! nowhere, and forgets it when the evaluator goes on.

use serde::Serialize;

use crate::model::recovery_code::RecoveryCode;

/// A recovery code, as the page shows it once.
#[derive(Serialize)]
pub struct RecoveryCodeView {
    /// Seven groups of five characters joined by dashes.
    pub code: String,
}

impl RecoveryCodeView {
    /// The view of a new code.
    ///
    /// Called by: the commands of `views/users/users.rs` that make a code.
    /// Calls: `RecoveryCode::to_text`.
    pub fn new(code: &RecoveryCode) -> RecoveryCodeView {
        RecoveryCodeView { code: code.to_text().to_string() }
    }
}
