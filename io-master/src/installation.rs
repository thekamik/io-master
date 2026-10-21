//! The installation of this Linux account, its one user and the
//! session of that user: the core of the protection of the program,
//! used by `main.rs` at the start, by the state of the window
//! (`tauri_ui/state.rs`) and by the views that log in, install and
//! change the password. What an installation is (its identifier and
//! role) is in `model/installation.rs`.
//!
//! ```text
//! lifecycle.rs   the first start, every later start, the removal
//! verify.rs      the check of the installation before the window opens
//! accounts.rs    the account: made at the first start, the login, the password
//! session.rs     the logged-in evaluator: their account and their open database
//! uninstall.rs   (not compiled: "DO NOT USE: TO BE REWORKED", step 12)
//! ```
//!
//! It uses `db/` (the system database, the keys of the databases),
//! `security/` (random bytes, the TPM) and `model/`; nothing above.

pub mod accounts;
pub mod lifecycle;
pub mod session;
pub mod verify;
