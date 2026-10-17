//! The direction of the dependencies between the folders of the
//! program (step 40 of docs/todo.md, docs/code_structure.md), checked
//! on the source itself, so that an auditor sees the rule and its check
//! in one place:
//!
//! ```text
//!               tauri_ui/
//!                  │
//!                  ▼
//!               business/
//!          ┌───────┼──────────────┐
//!          ▼       ▼              ▼
//!         ai/   hardware_tests/   │
//!          │       │              │
//!          └───────┴──────► db/ ◄─┘
//!                           │
//!          all of them ───► model/, security/
//! ```
//!
//! A folder uses only itself and the folders below it, and never a
//! folder above it: no cycle. The tests (`*_tests.rs`, `test_support.rs`)
//! are not held to it: they set up data across the layers.

use std::fs;
use std::path::Path;

/// What each folder may use besides itself: whole folders, or paths
/// that start so (`crate::` left out).
const ALLOWED: [(&str, &[&str]); 7] = [
    ("model", &[]),
    ("security", &[]),
    ("hardware_tests", &[]),
    ("db", &["model", "security"]),
    ("ai", &["db", "model", "security"]),
    ("business", &["ai", "db", "hardware_tests", "model", "security"]),
    (
        "tauri_ui",
        &[
            "business",
            "model",
            // The settings of the AI are resources of ai/api/ already
            // (connections, roles, checks): the tab AI shows them as
            // they are; and the presets of known services.
            "ai::api",
            "ai::presets",
            // The type of the open database, passed from the session to
            // the business, and the error of a database, turned into an
            // error of the page. Never a function of the CRUD of db/.
            "db::user::database::UserDb",
            "db::error",
            // The types of a port, shown in the page of the scan; the
            // scan itself goes through business/hardware/.
            "hardware_tests::devices::devices::PortFilter",
            "hardware_tests::devices::devices::PortType",
            "hardware_tests::devices::devices::SerialPortInfo",
            "hardware_tests::devices::devices::UsbInfo",
            "hardware_tests::devices::devices::AccessInfo",
            "hardware_tests::devices::port_property",
        ],
    ),
];

/// Every `crate::...` path in `text` (the path only, as far as it goes:
/// names joined by `::`, a list in braces cut off).
fn paths(text: &str) -> Vec<String> {
    let mut found = Vec::new();

    for (at, _) in text.match_indices("crate::") {
        let rest = &text[at + "crate::".len()..];
        let end = rest.find(|c: char| !(c.is_alphanumeric() || c == '_' || c == ':')).unwrap_or(rest.len());
        let path = rest[..end].trim_end_matches(':');

        // `crate::a::b::{...}`: the names in the braces are taken below.
        if rest[end..].starts_with('{') {
            continue;
        }

        if !path.is_empty() {
            found.push(path.to_owned());
        }
    }

    // `use crate::a::b::{C, D}` names C and D under a::b.
    for (at, _) in text.match_indices("use crate::") {
        let rest = &text[at + "use crate::".len()..];
        let Some(statement) = rest.split(';').next() else { continue };

        if let Some((prefix, list)) = statement.split_once("::{") {
            for name in list.trim_end_matches('}').split(',') {
                let name = name.split(" as ").next().unwrap_or_default().trim();

                if !name.is_empty() && name != "self" && !name.contains('{') {
                    found.push(format!("{prefix}::{name}"));
                }
            }
        }
    }

    found
}

/// The files of `folder` that are held to the rule (the folder and the
/// file of the same name next to it).
fn files(root: &Path, folder: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut pending = vec![root.join(folder)];

    if let Ok(text) = fs::read_to_string(root.join(format!("{folder}.rs"))) {
        found.push((format!("{folder}.rs"), text));
    }

    while let Some(directory) = pending.pop() {
        let Ok(entries) = fs::read_dir(&directory) else { continue };

        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();

            if path.is_dir() {
                pending.push(path);
            } else if name.ends_with(".rs") && !name.ends_with("_tests.rs") && name != "test_support.rs" {
                let shown = path.strip_prefix(root).unwrap_or(&path).display().to_string();
                found.push((shown, fs::read_to_string(&path).unwrap_or_default()));
            }
        }
    }

    found
}

#[test]
fn every_folder_uses_only_the_folders_below_it() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut broken = Vec::new();

    for (folder, allowed) in ALLOWED {
        let checked = files(&root, folder);
        assert!(!checked.is_empty(), "the folder {folder} has no files");

        for (file, text) in checked {
            for path in paths(&text) {
                let top = path.split("::").next().unwrap_or_default();

                if top == folder
                    || allowed.iter().any(|prefix| path == *prefix || path.starts_with(&format!("{prefix}::")))
                {
                    continue;
                }

                broken.push(format!("{file}: crate::{path}"));
            }
        }
    }

    broken.sort();
    broken.dedup();
    assert!(broken.is_empty(), "uses against the direction of the dependencies:\n{}", broken.join("\n"));
}
