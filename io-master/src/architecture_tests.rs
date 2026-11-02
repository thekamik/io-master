//! The direction of the dependencies of the program (`docs/code_structure.md`),
//! checked on the source itself, so that an auditor sees the rule and its
//! check in one place. Three groups (user, 2026-11-01), the dependencies one
//! way:
//!
//! ```text
//!   panels/          views/ (the commands the page calls) ──► session.rs, logic/ (what a panel shows)
//!      │
//!      ▼
//!   objects/         a folder per object: <object>_model.rs ◄── _store.rs ◄── _rules.rs ◄── _ai.rs
//!      │
//!      ▼
//!   infrastructure/  storage/, security/, pdf/, clock.rs, data_folder.rs: no object
//! ```
//!
//! A folder uses only itself and the folders below it, and never a folder
//! above it; inside `objects/`, a file uses only the roles at or below its
//! own (`the_roles_of_an_object_go_one_way`). The tests (the module `tests`
//! at the end of a file, `test_support.rs`) are not held to it: they set up
//! data across the layers.
//!
//! The tests of a file are its last item: only the part above
//! them is held to the rule, and a test checks that nothing of the program
//! follows them.

use std::fs;
use std::path::Path;

/// What each folder may use besides itself: whole folders, or paths
/// that start so (`crate::` left out). A folder is named by its path of
/// modules: `panels::views` and `panels::logic` are two layers of one
/// folder.
const ALLOWED: [(&str, &[&str]); 11] = [
    // infrastructure/: what belongs to no object and no panel, and knows
    // no object (user, 2026-11-01). The time of the computer, the data
    // folder, random bytes and the protection of the process, PDFs read
    // with Poppler (another program), the database files.
    ("infrastructure::clock", &[]),
    ("infrastructure::data_folder", &[]),
    ("infrastructure::security", &[]),
    ("infrastructure::pdf", &[]),
    // The database files; a new key of a database made of random bytes.
    ("infrastructure::storage", &["infrastructure::security"]),
    // objects/: every model with the functions that work on it.
    ("objects", &["infrastructure"]),
    // The API key of an integration: its store checks the integration it is
    // bound to.
    (
        "objects::ai::api_key",
        &[AI_ERROR, "objects::error", INTEGRATION_MODEL, INTEGRATION_STORE, "infrastructure::storage"],
    ),
    // The integration (the service, its connection) knows of the other
    // objects only the API key, masked in its errors, the model of the
    // journal (how an answer ended, `Ending`), the model of a chat (the
    // messages of a question, `Message`, `Speaker`) and the settings of the
    // user (the active integration); its rules refuse with the error of the
    // rules.
    (
        "objects::ai::integration",
        &[
            AI_ERROR,
            "objects::error",
            "objects::ai::api_key",
            CHAT_MODEL,
            JOURNAL_MODEL,
            "objects::user::program_data",
            "infrastructure",
        ],
    ),
    // The session: the user logged in, their open database, the progress
    // of the long work of the AI.
    ("panels::session", &["objects::user", "objects::session", "infrastructure"]),
    // What a panel shows: given the open database, it never reaches the
    // session. Of the integration it sees only what the panels see.
    ("panels::logic", &PANELS_SEE),
    // The commands of the page: they do their work with the folders below.
    ("panels::views", &PANELS_SEE_AND_THE_SESSION),
];

/// What of `objects/` and `infrastructure/` the panels may use: every object,
/// but of the AI of the integration only its question, its sending and the
/// check of an address, never the connection or a protocol.
const PANELS_SEE: [&str; 20] = [
    AI_ERROR,
    "objects::error",
    "objects::text",
    "objects::schema",
    "objects::ai::agent",
    "objects::ai::agent_roles",
    "objects::ai::api_key",
    "objects::ai::chat",
    "objects::ai::journal",
    "objects::ai::rag",
    INTEGRATION_MODEL,
    INTEGRATION_STORE,
    "objects::ai::integration::integration_ai::question",
    "objects::ai::integration::integration_ai::sending",
    "objects::ai::integration::integration_ai::check",
    "objects::ai::integration::integration_rules",
    "objects::evaluation",
    "objects::session",
    "objects::user",
    "infrastructure",
];

/// What the commands of the page may use: what the panels see, the session
/// and the work of the panels.
const PANELS_SEE_AND_THE_SESSION: [&str; 22] = [
    PANELS_SEE[0],
    PANELS_SEE[1],
    PANELS_SEE[2],
    PANELS_SEE[3],
    PANELS_SEE[4],
    PANELS_SEE[5],
    PANELS_SEE[6],
    PANELS_SEE[7],
    PANELS_SEE[8],
    PANELS_SEE[9],
    PANELS_SEE[10],
    PANELS_SEE[11],
    PANELS_SEE[12],
    PANELS_SEE[13],
    PANELS_SEE[14],
    PANELS_SEE[15],
    PANELS_SEE[16],
    PANELS_SEE[17],
    PANELS_SEE[18],
    PANELS_SEE[19],
    "panels::session",
    "panels::logic",
];

/// The parts of `objects/ai/` named in more than one row.
const AI_ERROR: &str = "objects::ai_error";
const INTEGRATION_MODEL: &str = "objects::ai::integration::integration_model";
const INTEGRATION_STORE: &str = "objects::ai::integration::integration_store";
const CHAT_MODEL: &str = "objects::ai::chat::chat_model";
const JOURNAL_MODEL: &str = "objects::ai::journal::journal_model";

/// Every `crate::...` path in `text` (the path only, as far as it goes:
/// names joined by `::`, a list in braces cut off).
fn paths(text: &str) -> Vec<String> {
    let mut found = Vec::new();

    for (at, _) in text.match_indices("crate::") {
        // `pub(in crate::a)` names where an item is seen, not what it uses.
        if text[..at].ends_with("(in ") {
            continue;
        }

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

/// The line that opens the tests of a file, its last item.
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
            } else if path.extension().is_some_and(|extension| extension == "rs") && name != "test_support.rs" {
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

/// The roles of the files of an object, from the bottom up (user,
/// 2026-11-01): `agent_model.rs`, `agent_store.rs`, `agent_rules.rs`,
/// `agent_ai.rs`, or a folder of the same name (`work_unit_ai/`).
const ROLES: [&str; 4] = ["_model", "_store", "_rules", "_ai"];

/// The files of `objects/` outside an object, with the role they serve:
/// the list of the tables of the stores, the error of the rules and the
/// rule of a text, the error of the AI (which holds the error of the rules).
const SHARED_ROLES: [(&str, usize); 4] = [("schema", 1), ("error", 2), ("text", 2), ("ai_error", 3)];

/// The role of a path of modules of `objects/`
/// (`objects::ai::agent::agent_store::read_agent` → 1): the first of its
/// names that ends with a role, or a shared file; `None` for none.
fn role_of(path: &str) -> Option<usize> {
    for name in path.split("::") {
        if let Some((_, rank)) = SHARED_ROLES.iter().find(|(shared, _)| *shared == name) {
            return Some(*rank);
        }

        if let Some(rank) = ROLES.iter().position(|role| name.ends_with(role)) {
            return Some(rank);
        }
    }

    None
}

#[test]
fn the_roles_of_an_object_go_one_way() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut broken = Vec::new();

    for (file, text) in files(&root, "objects") {
        let module = file.trim_end_matches(".rs").replace('/', "::");
        let own = role_of(&module);

        for path in paths(&text) {
            if !path.starts_with("objects::") {
                continue;
            }

            // A role uses only the roles at or below its own; a file of no
            // role uses no role.
            let allowed = match (own, role_of(&path)) {
                (_, None) => true,
                (None, Some(_)) => false,
                (Some(own), Some(used)) => used <= own,
            };

            if !allowed {
                broken.push(format!("{file}: crate::{path}"));
            }
        }
    }

    broken.sort();
    broken.dedup();
    assert!(broken.is_empty(), "uses against the roles of an object:\n{}", broken.join("\n"));
}

/// The folder where SQL may be written besides the stores: the database
/// files (CLAUDE.md, "The database").
const SQL_FOLDER: &str = "infrastructure/storage/";

/// The end of the name of the store of an object (`user_data_store.rs`), the
/// place where the SQL of a model is written.
const STORE_FILE: &str = "_store.rs";

/// The signs of SQL in a file: the library of SQLite, and the connection
/// of an open database.
const SQL_SIGNS: [&str; 2] = ["rusqlite", ".connection()"];

#[test]
fn the_sql_is_only_in_the_store() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut broken = Vec::new();

    for (file, text) in all_rust_files(&root) {
        let text = program_part(&text);

        // The files of tests name the signs of SQL to look for them.
        let store = file.starts_with("objects/") && file.ends_with(STORE_FILE);

        if store || FILES_OF_TESTS.contains(&file.as_str()) || file.starts_with(SQL_FOLDER) {
            continue;
        }

        for sign in SQL_SIGNS {
            if text.contains(sign) {
                broken.push(format!("{file}: {sign}"));
            }
        }
    }

    assert!(broken.is_empty(), "SQL outside the store:\n{}", broken.join("\n"));
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
/// test no one file.
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
