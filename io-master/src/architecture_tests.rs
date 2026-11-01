//! The direction of the dependencies between the folders of the
//! program (docs/code_structure.md), checked on the source itself, so that
//! an auditor sees the rule and its check in one place:
//!
//! ```text
//!   panels/views/   (the commands the page calls) ──► session/ (the user logged in)
//!         │
//!         ▼
//!   panels/logic/   (the work behind the panels: evaluation, settings)
//!         │
//!         ▼
//!     objects/      (a folder per object: its model, store, rules, AI; schema.rs)
//!         │
//!         ▼
//!        ai/        (models/: the AI on the models, a mirror of model/)
//!         │
//!         ▼
//!      domain/      (models/: the rules of the data, a mirror of model/)
//!         │
//!         ▼
//!        db/        (create, read and update of the models)
//!         │
//!         ▼
//!   model/                                (nothing of the program)
//!   infrastructure/  (storage/, security/, pdf/, clock.rs, data_folder.rs: no object)
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
//! The tests of a file are its last item: only the part above
//! them is held to the rule, and a test checks that nothing of the program
//! follows them.

use std::fs;
use std::path::Path;

/// What each folder may use besides itself: whole folders, or paths
/// that start so (`crate::` left out). A folder is named by its path of
/// modules: `panels::views` and `panels::logic` are two layers of one
/// folder.
const ALLOWED: [(&str, &[&str]); 23] = [
    ("model", &[]),
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
    // The stores not yet moved: the index of a document of the product is cut
    // and searched as the knowledge is (rag_store.rs).
    ("db", &[RAG_STORE, "infrastructure::storage", "model"]),
    // The rules of the data: they read and write through db/ and the stores
    // of the objects (an agent names an integration that exists).
    (
        "domain",
        &[
            AGENT_STORE,
            INTEGRATION_MODEL,
            INTEGRATION_STORE,
            "infrastructure::clock",
            "infrastructure::pdf",
            "infrastructure::storage",
            "db",
            "model",
        ],
    ),
    // The AI not yet moved into objects/: it reads the integration, its key
    // and the knowledge, and writes the journal, through objects/ai/.
    (
        "ai",
        &[
            AI_ERROR,
            AGENT,
            INTEGRATION,
            JOURNAL,
            "infrastructure::clock",
            "domain",
            "db",
            "infrastructure::storage",
            "model",
        ],
    ),
    // The chat asks through the integration.
    (
        "ai::models::ai::chat",
        &[
            AI_ERROR,
            AGENT,
            INTEGRATION,
            JOURNAL,
            "infrastructure::clock",
            "domain",
            "db",
            "infrastructure::storage",
            "model",
        ],
    ),
    // The locks of the session as a long work takes them.
    ("ai::lock", &[AI_ERROR, "db", "infrastructure::storage", "model"]),
    // The AI on a plan: the loops over its work units (evaluation_plan.rs)
    // use the work on one work unit (work_unit/), never the other way.
    (
        "ai::models::evaluation::work_unit",
        &[
            AI_ERROR,
            "ai::lock",
            "ai::models::ai",
            AGENT,
            INTEGRATION,
            JOURNAL,
            "domain",
            "db",
            "infrastructure::storage",
            "model",
        ],
    ),
    (
        "ai::models::evaluation::evaluation_plan",
        &[
            AI_ERROR,
            AGENT_STORE,
            INTEGRATION_STORE,
            "ai::lock",
            "ai::models::evaluation::work_unit",
            "ai::models::session",
            "domain",
            "db",
            "infrastructure::storage",
            "model",
        ],
    ),
    // The progress of a long work: a helper model, no database.
    ("ai::models::session::progress", &["model"]),
    // The work behind the panels, one folder per panel; a panel may use
    // another, in one direction only. It gets the open database as a parameter and never reaches
    // the session. Of the integration it sees the question, its sending and
    // the checks of an address and a certificate (the settings of the AI),
    // never the connection or a protocol.
    // The sources of the knowledge of an agent are read from PDFs too.
    (
        "panels::logic",
        &[
            AI_ERROR,
            API_KEY_MODEL,
            AGENT,
            RAG,
            CHAT,
            JOURNAL,
            QUESTION,
            SENDING,
            ADDRESS,
            ASSESSMENT_ANSWER,
            "objects::user",
            INTEGRATION_MODEL,
            INTEGRATION_STORE,
            API_KEY_STORE,
            "infrastructure::clock",
            "domain",
            "db",
            "infrastructure::storage",
            "model",
            "infrastructure::security",
            "infrastructure::pdf",
        ],
    ),
    // objects/: every model with the functions that work on it (user,
    // 2026-11-01). Its list of the tables names the stores still in db/;
    // AiError holds the error of the rules (domain/error.rs).
    ("objects", &["infrastructure", "db", "domain::error", "domain::text", "model"]),
    // The API key of an integration: its store checks the integration it is
    // bound to.
    ("objects::ai::api_key", &[AI_ERROR, INTEGRATION_MODEL, INTEGRATION_STORE, "infrastructure::storage"]),
    // The integration (the service, its connection) knows of the other
    // objects only the API key, masked in its errors, and the model of the
    // journal (how an answer ended, `Ending`).
    ("objects::ai::integration", &[AI_ERROR, "objects::ai::api_key", JOURNAL_MODEL, "infrastructure", "model"]),
    // The agent asks through the integration, with the knowledge of its
    // bases; the knowledge knows no other object.
    ("objects::ai::agent", &[AI_ERROR, INTEGRATION, "objects::ai::rag", "infrastructure", "db", "domain", "model"]),
    ("objects::ai::rag", &["infrastructure", "db", "domain", "model"]),
    // The journal writes what the integration asked and was answered.
    ("objects::ai::journal", &[AI_ERROR, INTEGRATION, "infrastructure", "db", "model"]),
    // The session: the user logged in and their open database.
    ("session", &["objects::user", "db", "infrastructure::storage", "model"]),
    // The commands of the page: they do their work with the folders below.
    (
        "panels::views",
        &[
            "panels::logic",
            AI_ERROR,
            AGENT,
            RAG,
            CHAT,
            JOURNAL,
            QUESTION,
            SENDING,
            PROGRESS,
            EVALUATION_PLAN,
            ASSESSMENT_QUESTION,
            FILLING,
            "objects::user",
            INTEGRATION_MODEL,
            INTEGRATION_STORE,
            API_KEY_STORE,
            "domain",
            "db",
            "infrastructure::storage",
            "model",
            "session",
        ],
    ),
];

/// The parts of the AI the other folders may use, named once for all of
/// them. Of the integration the panels see the question, its sending, the
/// check of an address, its model and its store, never the connection or a
/// protocol.
const AI_ERROR: &str = "objects::ai_error";
const INTEGRATION: &str = "objects::ai::integration";
const INTEGRATION_MODEL: &str = "objects::ai::integration::integration_model";
const INTEGRATION_STORE: &str = "objects::ai::integration::integration_store";
const API_KEY_STORE: &str = "objects::ai::api_key::api_key_store";
const JOURNAL: &str = "objects::ai::journal";
const JOURNAL_MODEL: &str = "objects::ai::journal::journal_model";
const API_KEY_MODEL: &str = "objects::ai::api_key::api_key_model";
const QUESTION: &str = "objects::ai::integration::integration_ai::question";
const SENDING: &str = "objects::ai::integration::integration_ai::sending";
const ADDRESS: &str = "objects::ai::integration::integration_ai::address";
const AGENT: &str = "objects::ai::agent";
const AGENT_STORE: &str = "objects::ai::agent::agent_store";
const RAG: &str = "objects::ai::rag";
const RAG_STORE: &str = "objects::ai::rag::rag_store";
const CHAT: &str = "ai::models::ai::chat";
const PROGRESS: &str = "ai::models::session::progress";
const EVALUATION_PLAN: &str = "ai::models::evaluation::evaluation_plan";
const ASSESSMENT_QUESTION: &str = "ai::models::evaluation::work_unit::assessment_question";
const ASSESSMENT_ANSWER: &str = "ai::models::evaluation::work_unit::assessment_answer";
const FILLING: &str = "ai::models::evaluation::work_unit::filling";

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

/// The folders where SQL may be written: the database files and the
/// models in them (CLAUDE.md, "The database").
const SQL_FOLDERS: [&str; 2] = ["infrastructure/storage/", "db/models/"];

/// The end of the name of the store of an object (`user_data_store.rs`), the
/// other place where SQL may be written.
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

        if store || FILES_OF_TESTS.contains(&file.as_str()) || SQL_FOLDERS.iter().any(|folder| file.starts_with(folder))
        {
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
