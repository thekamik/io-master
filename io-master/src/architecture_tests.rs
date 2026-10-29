//! The direction of the dependencies between the folders of the
//! program (docs/code_structure.md), checked on the source itself, so that
//! an auditor sees the rule and its check in one place:
//!
//! ```text
//!   panels/views/   (the commands the page calls) ──► session/ (the user logged in)
//!         │
//!         ▼
//!   panels/logic/   (the work behind the panels: program_start, evaluation_plan, ...)
//!         │
//!         ▼
//!        db/        (create, read and update of the models; the database files)
//!         │
//!         ▼
//!   model/, security/, pdf/     (nothing of the program)
//! ```
//!
//! **The program is being rebuilt (2026-10-28)**: the folders above `db/`
//! (the AI, the login, the views) come back one by one, each with its row
//! below, and the checks of the views and the components of the page with
//! the views.
//!
//! A folder uses only itself and the folders below it, and never a
//! folder above it: no cycle. The tests (the module `tests` at the end of
//! a file, `test_support.rs`) are not held to it: they set up data across
//! the layers.
//!
//! The tests of a file are its last item (step 44): only the part above
//! them is held to the rule, and a test checks that nothing of the program
//! follows them.

use std::fs;
use std::path::Path;

/// What each folder may use besides itself: whole folders, or paths
/// that start so (`crate::` left out). A folder is named by its path of
/// modules: `panels::views` and `panels::logic` are two layers of one
/// folder.
const ALLOWED: [(&str, &[&str]); 8] = [
    ("model", &[]),
    ("security", &[]),
    // Reading PDFs with Poppler: another program, nothing of ours.
    ("pdf", &[]),
    ("db", &["model", "security"]),
    // The only way to the AI: it reads the integration and its key, and
    // writes the journal.
    ("ai", &["db", "model"]),
    // The work behind the panels, one folder per panel; a panel may use
    // another (in one direction: chat and evaluation use rag and
    // evaluation_plan, never the other way). It gets the open database as a
    // parameter and never reaches the session.
    ("panels::logic", &["ai", "db", "model", "security"]),
    // The session: the user logged in and their open database.
    ("session", &["db", "model"]),
    // The commands of the page: they do their work with the folders below.
    ("panels::views", &["panels::logic", "ai", "db", "model", "session"]),
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
        let checked = files(&root, &folder.replace("::", "/"));
        assert!(!checked.is_empty(), "the folder {folder} has no files");

        for (file, text) in checked {
            for path in paths(&text) {
                let own = path == folder || path.starts_with(&format!("{folder}::"));

                if own || allowed.iter().any(|prefix| path == *prefix || path.starts_with(&format!("{prefix}::"))) {
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
const FILES_OF_TESTS: [&str; 1] = ["architecture_tests.rs"];

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
