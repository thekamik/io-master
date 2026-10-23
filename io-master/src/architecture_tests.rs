//! The direction of the dependencies between the folders of the
//! program (step 40 of docs/todo.md, docs/code_structure.md), checked
//! on the source itself, so that an auditor sees the rule and its check
//! in one place:
//!
//! ```text
//!                         views/
//!                  (one folder per view of the window)
//!                            │
//!                            ▼
//!                       components/
//!                  (the elements used more than once)
//!        ┌──────────┬────────┼─────────┬──────────────┬───────────┐
//!        ▼          ▼        ▼         ▼              ▼           ▼
//!      app/        ai/   hardware_  installation/    pdf/      (model/,
//!  (state, errors)  │     tests/        │                       security/)
//!        │          │                   │
//!        └──────────┴──────► db/ ◄──────┘
//!                            │
//!          all of them ───► model/, security/
//! ```
//!
//! A folder uses only itself and the folders below it, and never a
//! folder above it: no cycle. The tests (the module `tests` at the end of
//! a file, `test_support.rs`) are not held to it: they set up data across
//! the layers.
//!
//! The tests of a file are its last item (step 44): only the part above
//! them is held to the rule, and a test checks that nothing of the program
//! follows them. Two rules of step 43 (docs/architecture.md, section 10.2)
//! are checked as well: a view never uses another view (what two views need is a
//! component), and a component of the page never imports a view of the
//! page.

use std::fs;
use std::path::Path;

/// What each folder may use besides itself: whole folders, or paths
/// that start so (`crate::` left out).
const ALLOWED: [(&str, &[&str]); 10] = [
    ("model", &[]),
    ("security", &[]),
    ("hardware_tests", &[]),
    // Reading PDFs with Poppler: another program, nothing of ours.
    ("pdf", &[]),
    ("db", &["model", "security"]),
    ("ai", &["db", "model", "security"]),
    // The installation, its account and the session: the core of the
    // protection, below the window and the views.
    ("installation", &["db", "model", "security"]),
    (
        "app",
        &[
            "installation",
            "model",
            // The errors of the folders below, turned into errors of the
            // page; never a function of them.
            "ai::api::completions::AiError",
            "ai::api::connections::ConnectionSaveError",
            "ai::api::roles::ModelError",
            "db::error",
            "db::oscal::error::OscalError",
            "hardware_tests::devices::scan_settings::ScanSettingsError",
        ],
    ),
    // The components (components.rs): the elements used more than once,
    // built as the elements of a view; never a view.
    ("components", &["app", "ai", "db", "hardware_tests", "installation", "model", "pdf", "security"]),
    // The views (views.rs): the commands of the elements of the window,
    // on top of everything. They use the state and the errors of the
    // window (app/) and the components, and do their work with the folders
    // below. A view never uses another view (checked below).
    ("views", &["app", "ai", "components", "db", "hardware_tests", "installation", "model", "pdf", "security"]),
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

/// The line that opens the tests of a file, its last item (step 44).
const TESTS_START: &str = "\n#[cfg(test)]\nmod tests {\n";

/// The part of a file that is the program: all of it up to its tests.
fn program_part(text: &str) -> String {
    match text.find(TESTS_START) {
        Some(at) => text[..at].to_owned(),
        None => text.to_owned(),
    }
}

/// The files of `folder` that are held to the rule (the folder and the
/// file of the same name next to it).
fn files(root: &Path, folder: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut pending = vec![root.join(folder)];

    if let Ok(text) = fs::read_to_string(root.join(format!("{folder}.rs"))) {
        found.push((format!("{folder}.rs"), program_part(&text)));
    }

    while let Some(directory) = pending.pop() {
        let Ok(entries) = fs::read_dir(&directory) else { continue };

        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();

            if path.is_dir() {
                pending.push(path);
            } else if name.ends_with(".rs") && name != "test_support.rs" {
                let shown = path.strip_prefix(root).unwrap_or(&path).display().to_string();
                found.push((shown, program_part(&fs::read_to_string(&path).unwrap_or_default())));
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

/// The views of the window: the folders of `views/`.
fn view_folders(root: &Path) -> Vec<String> {
    let mut found = Vec::new();

    for entry in fs::read_dir(root.join("views")).expect("the folder views is read").flatten() {
        if entry.path().is_dir() {
            found.push(entry.file_name().to_string_lossy().into_owned());
        }
    }

    found
}

/// How deep the module of `file` lies in the folder of its view: 1 for
/// `chat/mod.rs`, 2 for `chat/conversation/mod.rs` and `chat/turn.rs`.
fn depth_in_view(file: &Path) -> usize {
    let parts = file.components().count();

    if file.file_name().is_some_and(|name| name == "mod.rs") { parts - 1 } else { parts }
}

/// The paths of `text` that start with `super::`, as the number of their
/// `super::` (a path `super::super::x` is 2).
fn super_chains(text: &str) -> Vec<usize> {
    let mut found = Vec::new();

    for (at, _) in text.match_indices("super::") {
        let before = text[..at].chars().next_back().unwrap_or(' ');

        if before == ':' || before.is_alphanumeric() || before == '_' {
            continue;
        }

        found.push(text[at..].split("::").take_while(|part| *part == "super").count());
    }

    found
}

#[test]
fn a_view_never_uses_another_view() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let views = view_folders(&root);
    let mut broken = Vec::new();

    assert!(views.len() > 1, "the views are found");

    for view in &views {
        for (file, text) in files(&root.join("views"), view) {
            // By its name: `crate::views::<another view>`.
            for path in paths(&text) {
                let mut parts = path.split("::");

                if parts.next() == Some("views") && parts.next().is_some_and(|other| other != view) {
                    broken.push(format!("views/{file}: crate::{path}"));
                }
            }

            // By `super::`: a chain that leaves the folder of the view.
            let depth = depth_in_view(Path::new(&file));

            for chain in super_chains(&text) {
                if chain >= depth {
                    broken.push(format!("views/{file}: a path with {chain} times super:: leaves the view {view}"));
                }
            }
        }
    }

    broken.sort();
    broken.dedup();
    assert!(broken.is_empty(), "a view uses another view:\n{}", broken.join("\n"));
}

#[test]
fn a_component_of_the_page_never_imports_a_view() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("ui").join("src").join("components");
    let mut pending = vec![root.clone()];
    let mut checked = 0;
    let mut broken = Vec::new();

    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory).expect("the folder of the components is read").flatten() {
            let path = entry.path();

            if path.is_dir() {
                pending.push(path);
                continue;
            }

            checked += 1;
            let text = fs::read_to_string(&path).unwrap_or_default();

            for line in text.lines() {
                if line.trim_start().starts_with("import") && line.contains("/views/") {
                    broken.push(format!("{}: {}", path.strip_prefix(&root).unwrap_or(&path).display(), line.trim()));
                }
            }
        }
    }

    assert!(checked > 0, "the components of the page are found");
    assert!(broken.is_empty(), "a component imports a view:\n{}", broken.join("\n"));
}

/// Every `.rs` file under `root`.
fn all_rust_files(root: &Path) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];

    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory).expect("the folder is read").flatten() {
            let path = entry.path();

            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                let shown = path.strip_prefix(root).unwrap_or(&path).display().to_string();
                found.push((shown, fs::read_to_string(&path).unwrap_or_default()));
            }
        }
    }

    found
}

/// The words that begin an item of Rust at the left edge of a line.
const ITEM_STARTS: [&str; 13] = [
    "fn ",
    "pub ",
    "use ",
    "impl",
    "struct ",
    "enum ",
    "const ",
    "static ",
    "mod ",
    "#[",
    "type ",
    "trait ",
    "macro_rules!",
];

#[test]
fn the_tests_of_a_file_are_its_last_item() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut broken = Vec::new();
    let mut with_tests = 0;

    for (file, text) in all_rust_files(&root) {
        let starts = text.matches(TESTS_START).count();

        if starts == 0 {
            continue;
        }

        with_tests += 1;

        if starts > 1 {
            broken.push(format!("{file}: more than one module of tests"));
            continue;
        }

        let after = text.split_once(TESTS_START).map(|(_, rest)| rest).unwrap_or_default();
        let last = after.lines().rfind(|line| !line.trim().is_empty()).unwrap_or_default();

        if last != "}" {
            broken.push(format!("{file}: the file does not end with the module of tests"));
        }

        for line in after.lines() {
            if ITEM_STARTS.iter().any(|start| line.starts_with(start)) {
                broken.push(format!("{file}: code of the program after the tests: {line}"));
            }
        }
    }

    assert!(with_tests > 0, "files with their tests are found");
    assert!(broken.is_empty(), "the tests are not the last item:\n{}", broken.join("\n"));
}

/// The files of tests of their own: the tests of the whole program, which
/// test no one file (step 44).
const FILES_OF_TESTS: [&str; 2] = ["architecture_tests.rs", "command_lists_tests.rs"];

#[test]
fn the_tests_of_a_file_are_never_in_a_file_of_their_own() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut found = Vec::new();

    for (file, _) in all_rust_files(&root) {
        let name = Path::new(&file).file_name().unwrap_or_default().to_string_lossy().into_owned();

        if name.ends_with("_tests.rs") && !FILES_OF_TESTS.contains(&name.as_str()) {
            found.push(file);
        }
    }

    assert!(
        found.is_empty(),
        "tests in a file of their own (they belong at the end of the file they test):\n{}",
        found.join("\n")
    );
}
