//! The recovery code as the page gets it, once: after the first start, a
//! recovery with the code, a change of the password, and a new code asked
//! for. It is the one secret that reaches the page (CLAUDE.md, "The window
//! and the layers"); the page shows it and forgets it when the user goes on.
//!
//! ```text
//! RecoveryCode ──to_text──► Zeroizing<String> ──serialize──► {"code": "ABCDE-FGHJK-..."}
//! ```
//!
//! The text is written into the answer straight from its wiped memory,
//! without a plain copy that would stay behind.

use serde::ser::{Serialize, SerializeStruct, Serializer};
use zeroize::Zeroizing;

use crate::objects::user::user_keys::user_keys_rules::recovery_code::RecoveryCode;

/// The recovery code, as the page shows it: seven groups of five
/// characters ("ABCDE-FGHJK-..."); wiped from memory when it is dropped,
/// after it was sent to the page.
pub struct RecoveryCodeView {
    code: Zeroizing<String>,
}

impl RecoveryCodeView {
    /// The view of a new code.
    ///
    /// Called by: `panels/views/program_start/installation.rs`: `installation_create`;
    /// `panels/views/program_start/login.rs`: `login_recover`;
    /// `panels/views/account/credentials.rs`: `account_change_password`,
    /// `account_new_recovery_code`.
    /// Calls: `objects/user/user_keys/user_keys_rules/recovery_code.rs`: `RecoveryCode::to_text`.
    pub fn new(code: &RecoveryCode) -> RecoveryCodeView {
        RecoveryCodeView { code: code.to_text() }
    }
}

impl Serialize for RecoveryCodeView {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut view = serializer.serialize_struct("RecoveryCodeView", 1)?;

        view.serialize_field("code", self.code.as_str())?;
        view.end()
    }
}
