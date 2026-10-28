//! The first start, as the page calls it (`ui/src/views/program_start/`):
//! the one user of the installation, made with a password given twice. The
//! recovery code comes back to the page once, to be shown and written
//! down; it is kept nowhere, so it can never be shown again.
//!
//! ```text
//! InstallationForm ──installation_create(name, password, password repeated)──►
//!     program_start::create_account ──► Session::log_in ──► the recovery code, once
//! ```

use serde::Serialize;
use tauri::State;
use zeroize::Zeroizing;

use crate::business_logic::program_start::access::create_account;
use crate::session::state::Session;
use crate::views::error::ViewError;

/// The recovery code, as the page shows it: seven groups of five
/// characters ("ABCDE-FGHJK-...").
#[derive(Serialize)]
pub struct RecoveryCodeView {
    code: String,
}

/// Makes the installation and its one user, logs them in, and gives the
/// recovery code. Run apart from the window (`async`): wrapping the key
/// with the password takes a moment by design (Argon2id). The passwords
/// are wiped from memory when the command ends.
///
/// Called by: `InstallationForm.svelte`, "Install".
/// Calls: `program_start::access::create_account`, `Session::log_in`.
#[tauri::command(async)]
pub fn installation_create(
    session: State<'_, Session>,
    name: String,
    password: String,
    password_repeated: String,
) -> Result<RecoveryCodeView, ViewError> {
    let password = Zeroizing::new(password);
    let password_repeated = Zeroizing::new(password_repeated);
    let created = create_account(session.data_folder(), &name, &password, &password_repeated)?;
    let code = created.code.to_text();

    session.log_in(created.user, created.database)?;

    Ok(RecoveryCodeView { code: code.as_str().to_owned() })
}
