//! The state of the program as the page gets it: whether the program
//! is installed, who is logged in, which pages exist, the warnings
//! every page shows at the top, and what the tab Program of Settings
//! shows about the installation.

use std::path::Path;

use serde::Serialize;

use crate::installation::accounts::Account;
use crate::installation::permissions::{self, PermissionProblem};

/// The state of the program, as the page sees it.
#[derive(Debug, Serialize)]
pub struct ProgramView {
    /// `false` at the first start: the page shows the setup.
    pub installed: bool,
    /// The name of the logged-in evaluator; `None`: the login page.
    pub user: Option<String>,
    /// The pages, in the order of the navigation.
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
    /// What the page calls it: `"chat"`, `"settings"`, ...
    pub id: &'static str,
    pub title: &'static str,
}

/// A bar at the top of every page.
#[derive(Debug, Serialize)]
pub struct WarningView {
    /// `"danger"` (red) or `"warning"` (orange).
    pub level: &'static str,
    pub text: String,
}

/// The pages of the navigation.
fn pages() -> Vec<PageView> {
    vec![
        PageView { id: "chat", title: "Chat" },
        PageView { id: "converter", title: "Converter" },
        PageView { id: "evaluation_plan", title: "Evaluation plan" },
        PageView { id: "evaluation_lab", title: "Evaluation lab" },
        PageView { id: "knowledge", title: "Knowledge (RAG)" },
        PageView { id: "users", title: "User" },
        PageView { id: "settings", title: "Settings" },
    ]
}

/// The warnings: the folder of the installation open to other accounts or
/// belonging to another one (`installation/permissions.rs`), with the
/// command that corrects it. (Every account has its recovery code from the
/// setup on, step 49 of docs/history.md, so no bar is needed for that.)
///
/// Called by: [`ProgramView::new`].
/// Calls: `permissions::problems`.
fn warnings(data_dir: &Path) -> Vec<WarningView> {
    let problems = permissions::problems(data_dir);
    let mut warnings = Vec::new();
    let mut open = 0;

    for problem in &problems {
        match problem {
            PermissionProblem::Owner(path) => warnings.push(WarningView {
                level: "danger",
                text: format!("The folder {} belongs to another account of this computer.", path.display()),
            }),
            PermissionProblem::Open(_) => open += 1,
        }
    }

    if open > 0 {
        warnings.push(WarningView {
            level: "danger",
            text: format!(
                "Other accounts of this computer may open {open} item(s) of the folder of IO Master. Close it: chmod -R go-rwx {}",
                data_dir.display()
            ),
        });
    }

    warnings
}

impl ProgramView {
    /// The view of the program: whether it is installed, the logged-in
    /// evaluator and the folder of the installation.
    pub fn new(installed: bool, user: Option<Account>, data_dir: &Path) -> ProgramView {
        if !installed {
            return ProgramView {
                installed: false,
                user: None,
                pages: Vec::new(),
                warnings: Vec::new(),
                version: env!("CARGO_PKG_VERSION"),
                data_dir: data_dir.display().to_string(),
            };
        }

        ProgramView {
            installed: true,
            user: name_of(user),
            pages: pages(),
            warnings: warnings(data_dir),
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
    //! (`ui/src/views/shell/Shell.svelte`): every page has its icon,
    //! its subtitle and its view there, and the frame has nothing for a page
    //! that does not exist. The ids are the contract between the two projects,
    //! as the names of the commands are (`command_lists_tests.rs`); a page
    //! renamed on one side only loses its icon, subtitle or view in silence
    //! (found in step 43.4: the subtitle of a page).

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

    /// The ids of the pages.
    fn page_ids() -> Vec<String> {
        let mut ids = Vec::new();

        for page in pages() {
            ids.push(page.id.to_owned());
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
