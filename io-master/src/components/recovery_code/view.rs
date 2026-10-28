//! The data of the component "recovery code": the code shown once, after
//! the setup, a change or a recovery of the password, and after "Make a new
//! recovery code" (step 49 of docs/history.md). The one secret that reaches the
//! page (CLAUDE.md, "State lives in the core"): the page shows it, keeps it
//! nowhere, and forgets it when the evaluator goes on.

use serde::Serialize;
use zeroize::Zeroizing;

use crate::model_old::recovery_code::RecoveryCode;

/// A recovery code, as the page shows it once.
#[derive(Serialize)]
pub struct RecoveryCodeView {
    /// Seven groups of five characters joined by dashes; wiped from memory
    /// when the view is dropped, after it was sent to the page.
    #[serde(serialize_with = "secret_text")]
    pub code: Zeroizing<String>,
}

/// Writes the text of a secret into the answer to the page without a copy
/// of its own that would stay in memory.
///
/// Called by: serde, for [`RecoveryCodeView`].
fn secret_text<S: serde::Serializer>(text: &Zeroizing<String>, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(text)
}

impl RecoveryCodeView {
    /// The view of a new code.
    ///
    /// Called by: the commands of `views/users/users.rs` that make a code.
    /// Calls: `RecoveryCode::to_text`.
    pub fn new(code: &RecoveryCode) -> RecoveryCodeView {
        RecoveryCodeView { code: code.to_text() }
    }
}
