//! The state of the program as the page gets it: whether the program
//! is installed, who is logged in, which pages exist, the warnings
//! every page shows at the top, and what the tab Program of Settings
//! shows about the installation.

use std::path::Path;

use serde::Serialize;

use crate::installation::accounts::Account;
use crate::model::installation::Role;

/// The state of the program, as the page sees it.
#[derive(Debug, Serialize)]
pub struct ProgramView {
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
    /// The version of the program (`Cargo.toml`): "0.1.0".
    pub version: &'static str,
    /// The folder of the installation (not a secret: its databases
    /// are encrypted).
    pub data_dir: String,
}

/// A page in the navigation.
#[derive(Debug, Serialize)]
pub struct PageView {
    /// What the page calls it: `"chat"`, `"devices"`, ...
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
            PageView { id: "evaluation_plan", title: "Evaluation plan" },
            // A placeholder until step 41.5 of docs/todo.md; it holds the
            // documentation of the manufacturer as well (step 30).
            PageView { id: "evaluation_lab", title: "Evaluation lab" },
            PageView { id: "devices", title: "Devices" },
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

impl ProgramView {
    /// The view of the program with the given role (`None` before
    /// the setup), logged-in evaluator and folder of the installation.
    pub fn new(role: Option<Role>, user: Option<Account>, data_dir: &Path) -> ProgramView {
        let Some(role) = role else {
            return ProgramView {
                installed: false,
                role: None,
                user: None,
                pages: Vec::new(),
                warnings: Vec::new(),
                version: env!("CARGO_PKG_VERSION"),
                data_dir: data_dir.display().to_string(),
            };
        };

        ProgramView {
            installed: true,
            role: Some(role.name()),
            user: name_of(user),
            pages: pages(role),
            warnings: warnings(),
            version: env!("CARGO_PKG_VERSION"),
            data_dir: data_dir.display().to_string(),
        }
    }
}

/// The name of the logged-in evaluator, if somebody is.
fn name_of(user: Option<Account>) -> Option<String> {
    Some(user?.name)
}

#[cfg(test)]
mod tests {
    //! The pages of the navigation agree with the page of the frame
    //! (`ui/src/views/shell/Shell.svelte`): every page of a role has its icon,
    //! its subtitle and its view there, and the frame has nothing for a page
    //! that does not exist. The ids are the contract between the two projects,
    //! as the names of the commands are (`command_lists_tests.rs`); a page
    //! renamed on one side only loses its icon, subtitle or view in silence
    //! (found in step 43.4: the subtitle of Devices).

    use super::*;

    /// The source of the frame of the window.
    const SHELL: &str = include_str!("../../../../ui/src/views/shell/Shell.svelte");

    /// Every id of `prefix"<id>"` in the frame (`name === "`, `active === "`).
    fn ids_after(prefix: &str) -> Vec<String> {
        let mut found = Vec::new();

        for (at, _) in SHELL.match_indices(prefix) {
            let rest = &SHELL[at + prefix.len()..];
            found.push(rest.split('"').next().unwrap_or_default().to_owned());
        }

        found
    }

    /// The keys of the subtitles: the lines `  <id>: "..."` of the object
    /// `subtitles`.
    fn subtitle_keys() -> Vec<String> {
        let start = SHELL.find("const subtitles").expect("the frame has its subtitles");
        let end = start + SHELL[start..].find("};").expect("the subtitles end");

        SHELL[start..end]
            .lines()
            .skip(1)
            .filter_map(|line| line.trim().split_once(": \""))
            .map(|(key, _)| key.to_owned())
            .collect()
    }

    /// The ids of the pages of both roles.
    fn page_ids() -> Vec<String> {
        let mut ids = Vec::new();

        for role in [Role::Workstation, Role::Master] {
            for page in pages(role) {
                ids.push(page.id.to_owned());
            }
        }

        ids
    }

    #[test]
    fn every_page_has_its_icon_its_subtitle_and_its_view_in_the_frame() {
        let icons = ids_after("name === \"");
        let subtitles = subtitle_keys();
        let views = ids_after("active === \"");

        for id in page_ids() {
            assert!(icons.contains(&id), "the frame has no icon for the page {id}");
            assert!(subtitles.contains(&id), "the frame has no subtitle for the page {id}");
            assert!(views.contains(&id), "the frame shows no view for the page {id}");
        }
    }

    #[test]
    fn the_frame_has_nothing_for_a_page_that_does_not_exist() {
        let ids = page_ids();
        let mut found = ids_after("active === \"");
        found.extend(subtitle_keys());
        // The icons, except the one of the warning bars, which is no page.
        found.extend(ids_after("name === \"").into_iter().filter(|name| name != "warning"));

        assert!(!found.is_empty());

        for id in found {
            assert!(ids.contains(&id), "the frame names {id}, which is no page");
        }
    }
}
