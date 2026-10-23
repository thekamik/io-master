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
