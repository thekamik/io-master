//! Unit tests of `commands.rs`: a command is callable only when it is
//! listed in four places (the top of `commands.rs`); a mistake there
//! closes the door, but it is a mistake. This test reads the four lists
//! and says what one of them lacks.

use std::collections::BTreeSet;

/// The commands defined here: every function after `#[tauri::command]`.
fn defined() -> BTreeSet<String> {
    let source = include_str!("commands.rs");
    let mut names = BTreeSet::new();
    let mut lines = source.lines();

    while let Some(line) = lines.next() {
        if line.trim() == "#[tauri::command]"
            && let Some(next) = lines.next()
            && let Some(rest) = next.split("fn ").nth(1)
        {
            names.insert(rest.split(['(', '<']).next().unwrap_or_default().to_owned());
        }
    }

    names
}

/// The commands registered with Tauri in `main.rs`: `commands::<name>,`
/// inside `generate_handler!`.
fn registered() -> BTreeSet<String> {
    let source = include_str!("../main.rs");
    let start = source.find("generate_handler![").expect("main.rs registers the commands");
    let end = start + source[start..].find(']').expect("the list ends");

    source[start..end]
        .lines()
        .filter_map(|line| line.trim().strip_prefix("commands::"))
        .map(|name| name.trim_end_matches(',').to_owned())
        .collect()
}

/// The commands given a permission in `build.rs`.
fn in_manifest() -> BTreeSet<String> {
    let source = include_str!("../../build.rs");
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
    let source = include_str!("../../capabilities/default.json");
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
        ("main.rs (generate_handler!)", registered()),
        ("build.rs (AppManifest)", in_manifest()),
        ("capabilities/default.json", allowed()),
    ] {
        let missing: Vec<_> = defined.difference(&list).collect();
        let extra: Vec<_> = list.difference(&defined).collect();

        assert!(missing.is_empty() && extra.is_empty(), "{place}: missing {missing:?}, not a command {extra:?}");
    }
}
