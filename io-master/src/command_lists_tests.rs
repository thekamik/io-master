//! The commands of the window are callable only when they are listed in
//! four places: the `commands.rs` of an element of `views/`, `routes.rs`
//! (`generate_handler!`), `build.rs` (`AppManifest`) and
//! `capabilities/default.json`; a mistake there closes the door, but it
//! is a mistake. This test reads the four lists
//! and says what one of them lacks. It also checks the rule of the views
//! (`views.rs`): a command of an element lives in the folder of that
//! element and is named after it (`ports_table_scan` in
//! `views/scan/ports_table/commands.rs`).

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// The source folder of the program.
fn src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// The commands defined in `source`: every function after
/// `#[tauri::command]` or `#[tauri::command(async)]`.
fn commands_in(source: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut lines = source.lines();

    while let Some(line) = lines.next() {
        if line.trim().starts_with("#[tauri::command")
            && let Some(next) = lines.next()
            && let Some(rest) = next.split("fn ").nth(1)
        {
            names.push(rest.split(['(', '<']).next().unwrap_or_default().to_owned());
        }
    }

    names
}

/// Every `commands.rs` under `views/`, with the name of the folder of its
/// element.
fn element_files() -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut pending = vec![src().join("views")];

    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory).expect("the folder is read").flatten() {
            let path = entry.path();

            if path.is_dir() {
                pending.push(path);
            } else if path.file_name().is_some_and(|name| name == "commands.rs") {
                let element = directory.file_name().unwrap_or_default().to_string_lossy().into_owned();
                found.push((element, fs::read_to_string(&path).expect("the file is read")));
            }
        }
    }

    found
}

/// The commands defined in the views: since step 42 every command is in
/// the folder of its element.
fn defined() -> BTreeSet<String> {
    let mut names = BTreeSet::new();

    for (_, source) in element_files() {
        names.extend(commands_in(&source));
    }

    names
}

/// The commands registered with Tauri in `routes.rs`: the last name of each
/// path inside `generate_handler!`.
fn registered() -> BTreeSet<String> {
    let source = include_str!("routes.rs");
    let start = source.find("generate_handler![").expect("routes.rs registers the commands");
    let end = start + source[start..].find(']').expect("the list ends");

    source[start..end]
        .lines()
        .map(str::trim)
        .filter(|line| line.ends_with(',') && !line.starts_with("//"))
        .filter_map(|line| line.trim_end_matches(',').rsplit("::").next())
        .map(str::to_owned)
        .collect()
}

/// The commands given a permission in `build.rs`.
fn in_manifest() -> BTreeSet<String> {
    let source = include_str!("../build.rs");
    let start = source.find(".commands(&[").expect("build.rs lists the commands");
    let end = start + source[start..].find("])").expect("the list ends");

    source[start..end]
        .lines()
        .filter_map(|line| line.trim().strip_prefix('"'))
        .map(|name| name.trim_end_matches(',').trim_end_matches('"').to_owned())
        .collect()
}

/// The commands whose permission the window has in
/// `capabilities/default.json` (`allow-<name with dashes>`).
fn allowed() -> BTreeSet<String> {
    let source = include_str!("../capabilities/default.json");
    let json: serde_json::Value = serde_json::from_str(source).expect("the capability is JSON");

    json["permissions"]
        .as_array()
        .expect("a list of permissions")
        .iter()
        .filter_map(|permission| permission.as_str()?.strip_prefix("allow-"))
        .map(|name| name.replace('-', "_"))
        .collect()
}

#[test]
fn the_four_lists_of_the_commands_agree() {
    let defined = defined();

    assert!(!defined.is_empty());

    for (place, list) in [
        ("routes.rs (generate_handler!)", registered()),
        ("build.rs (AppManifest)", in_manifest()),
        ("capabilities/default.json", allowed()),
    ] {
        let missing: Vec<_> = defined.difference(&list).collect();
        let extra: Vec<_> = list.difference(&defined).collect();

        assert!(missing.is_empty() && extra.is_empty(), "{place}: missing {missing:?}, not a command {extra:?}");
    }
}

#[test]
fn a_command_of_a_view_is_named_after_its_element() {
    let elements = element_files();

    assert_ne!(elements.len(), 0);

    for (element, source) in elements {
        for command in commands_in(&source) {
            assert!(command.starts_with(&format!("{element}_")), "{command} is not a command of the element {element}");
        }
    }
}
