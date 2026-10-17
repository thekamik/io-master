//! What the page needs to know to show itself: whether the program
//! is installed, who is logged in, which pages exist, the warnings
//! every page shows at the top, and what the tab Program of Settings
//! shows about the installation.

use std::path::Path;

use serde::Serialize;

use crate::business::session::accounts::Account;
use crate::model::installation::Role;

/// The state of the program, as the page sees it.
#[derive(Debug, Serialize)]
pub struct AppView {
    /// `false` at the first start: the page shows the setup.
    pub installed: bool,
    /// `"workstation"` or `"master"`; `None` before the setup.
    pub role: Option<&'static str>,
    /// The name of the logged-in evaluator; `None`: the login page.
    pub user: Option<String>,
    /// The pages of the role, in the order of the navigation.
    pub pages: Vec<PageView>,
    /// The bars at the top of every page, the most serious first.
    pub warnings: Vec<WarningView>,
    /// The version of the program: "0.4.0".
    pub version: &'static str,
    /// The folder of the installation (not a secret: its databases
    /// are encrypted).
    pub data_dir: String,
}

/// A page in the navigation.
#[derive(Debug, Serialize)]
pub struct PageView {
    /// What the page calls it: `"chat"`, `"scan"`, ...
    pub id: &'static str,
    pub title: &'static str,
}

/// A bar at the top of every page.
#[derive(Debug, Serialize)]
pub struct WarningView {
    /// `"danger"` (red) or `"warning"` (orange).
    pub level: &'static str,
    pub text: &'static str,
}

/// The pages of a role.
fn pages(role: Role) -> Vec<PageView> {
    match role {
        Role::Workstation => vec![
            PageView { id: "chat", title: "Chat" },
            PageView { id: "evaluation", title: "Evaluation" },
            PageView { id: "schemes", title: "Create evaluation scheme" },
            // A placeholder until step 30 of docs/todo.md is designed.
            PageView { id: "documentation", title: "Manufacturer documentation" },
            PageView { id: "scan", title: "Scan and permissions" },
            PageView { id: "settings", title: "Settings" },
        ],
        Role::Master => vec![PageView { id: "admin", title: "Password reset" }],
    }
}

/// The warnings. Both are always shown so far: neither the TPM
/// factor nor the reset service can be configured yet.
///
/// TODO (step 8 of docs/todo.md): the red bar only while the TPM
/// factor is off; its text once the secret of the application
/// exists (step 13). TODO (step 11): the orange bar only while no
/// reset service is configured.
fn warnings() -> Vec<WarningView> {
    vec![
        WarningView {
            level: "danger",
            text: "No TPM is configured: the databases are protected by the passwords alone. \
                   Whoever copies the files can try to guess them.",
        },
        WarningView {
            level: "warning",
            text: "No password reset is configured: a forgotten password means lost data.",
        },
    ]
}

impl AppView {
    /// The view of the program with the given role (`None` before
    /// the setup), logged-in evaluator and folder of the installation.
    pub fn new(role: Option<Role>, user: Option<&Account>, data_dir: &Path) -> AppView {
        AppView {
            installed: role.is_some(),
            role: role.map(Role::name),
            user: user.map(|account| account.name.clone()),
            pages: role.map(pages).unwrap_or_default(),
            warnings: if role.is_some() { warnings() } else { Vec::new() },
            version: env!("CARGO_PKG_VERSION"),
            data_dir: data_dir.display().to_string(),
        }
    }
}
