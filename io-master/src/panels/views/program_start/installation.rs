//! The first start, as the page calls it (`ui/src/views/program_start/`):
//! the one user of the installation, made with a password given twice. The
//! recovery code comes back to the page once, to be shown and written
//! down; it is kept nowhere, so it can never be shown again.
//!
//! ```text
//! InstallationForm ──installation_create(name, password, password repeated)──►
//!     program_start::create_account ──► Session::log_in ──► the recovery code, once
//! ```

use tauri::State;
use zeroize::Zeroizing;

use crate::objects::user::user_data::user_data_rules::create_account;
use crate::panels::session::Session;
use crate::panels::views::error::ViewError;
use crate::panels::views::recovery_code::RecoveryCodeView;

/// Makes the installation and its one user, logs them in, and gives the
/// recovery code. Run apart from the window (`async`): wrapping the key
/// with the password takes a moment by design (Argon2id). The passwords
/// are wiped from memory when the command ends.
///
/// Input: `session`, the session of the window, nobody logged in; `name`,
/// the name typed; `password`, `password_repeated`, the password typed twice.
/// Output: the recovery code (`{"code": "ABCDE-..."}`); the refusals of the
/// account (`ViewError::Account`), an error of the session.
///
/// Called by: `ui/src/lib/api/program_start.ts`: `installationCreate`, from
/// `ui/src/views/program_start/InstallationForm.svelte`, "Install".
/// Calls: `objects/user/user_data/user_data_rules.rs`: `create_account`; `panels/session.rs`: `Session::log_in`;
/// `panels/views/recovery_code.rs`: `RecoveryCodeView::new`.
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
    let view = RecoveryCodeView::new(&created.code);

    session.log_in(created.user, created.database)?;

    Ok(view)
}
