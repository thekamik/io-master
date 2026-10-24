//! The commands of the window are callable only when they are listed in
//! four places: the entry of their view (`views/<view>/<view>.rs`) or of
//! their component, `routes.rs` (`generate_handler!`), `build.rs`
//! (`AppManifest`) and `capabilities/default.json`; a mistake there closes
//! the door, but it is a mistake. This test reads the four lists and says
//! what one of them lacks. It also checks that a command is named after an
//! element of its view (`chat_list_get` in `views/chat/chat.rs`, the
//! element `views/chat/chat_list/`) or after its component
//! (`pdf_viewer_get`).

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

/// The files that define commands, under `views/` and `components/`: the
/// entry of a view or a component, the file named after its folder (step
/// 46), each with the names its commands may begin with: the names of the
/// elements of the view (the folders beside the entry, and the components
/// of its page) and the name of the folder itself, for a component.
fn element_files() -> Vec<(Vec<String>, String)> {
    let mut found = Vec::new();
    let mut pending = vec![src().join("views"), src().join("components")];

    while let Some(directory) = pending.pop() {
        let folder = directory.file_name().unwrap_or_default().to_string_lossy().into_owned();
        let mut elements = vec![folder.clone()];
        let mut files = Vec::new();

        for entry in fs::read_dir(&directory).expect("the folder is read").flatten() {
            let path = entry.path();

            if path.is_dir() {
                elements.push(path.file_name().unwrap_or_default().to_string_lossy().into_owned());
                pending.push(path);
            } else {
                files.push(path);
            }
        }

        for path in files {
            let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();

            if name == format!("{folder}.rs") {
                let mut names = elements.clone();
                names.extend(page_elements(&directory));
                found.push((names, fs::read_to_string(&path).expect("the file is read")));
            }
        }
    }

    found
}

/// The elements of a view on the page: its components
/// (`ui/src/views/<view>/LoginForm.svelte` is the element `login_form`), for
/// the elements that have no folder in the program (no data of their own).
fn page_elements(directory: &Path) -> Vec<String> {
    let relative = directory.strip_prefix(src()).unwrap_or(directory);
    let page = Path::new(env!("CARGO_MANIFEST_DIR")).join("ui").join("src").join(relative);
    let mut names = Vec::new();

    for entry in fs::read_dir(page).into_iter().flatten().flatten() {
        let file = entry.file_name().to_string_lossy().into_owned();

        if let Some(component) = file.strip_suffix(".svelte") {
            names.push(snake_case(component));
        }
    }

    names
}

/// `LoginForm` as `login_form`.
fn snake_case(name: &str) -> String {
    let mut snake = String::new();

    for (index, letter) in name.chars().enumerate() {
        if letter.is_uppercase() && index > 0 {
            snake.push('_');
        }
        snake.extend(letter.to_lowercase());
    }

    snake
}

/// The commands defined in the views and the components: since step 42
/// every command is in the folder of its element (or component, step 43).
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
    let files = element_files();

    assert_ne!(files.len(), 0);

    for (elements, source) in files {
        for command in commands_in(&source) {
            let named = elements.iter().any(|element| command.starts_with(&format!("{element}_")));

            assert!(named, "{command} is named after none of {elements:?}");
        }
    }
}
