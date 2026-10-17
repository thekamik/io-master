//! Unit tests of `rules.rs`.

use super::*;

use crate::model::test_projects::{External, OutputSection, PageSpan, ProposalState, Source};

fn link(id: i64, from: i64, to: i64) -> Link {
    Link { id, from, to }
}

/// Tiles: 1 input, 2 requirement, 3 output, 4 input, 5 requirement.
fn tiles() -> Vec<(i64, TileKind)> {
    vec![
        (1, TileKind::Input),
        (2, TileKind::Requirement),
        (3, TileKind::Output),
        (4, TileKind::Input),
        (5, TileKind::Requirement),
    ]
}

#[test]
fn the_allowed_links_have_their_meaning() {
    let tiles = tiles();

    assert_eq!(check_link(&tiles, &[], 1, 2), Ok(LinkMeaning::NeededBy));
    assert_eq!(check_link(&tiles, &[], 2, 3), Ok(LinkMeaning::ReportedIn));
    assert_eq!(check_link(&tiles, &[], 5, 4), Ok(LinkMeaning::Provides));
    assert_eq!(check_link(&tiles, &[], 2, 5), Ok(LinkMeaning::Precedes));
}

#[test]
fn other_links_are_refused() {
    let tiles = tiles();

    for (from, to) in [(1, 3), (1, 4), (3, 2), (3, 1)] {
        assert!(matches!(check_link(&tiles, &[], from, to), Err(SchemeError::LinkNotAllowed(..))), "{from} -> {to}");
    }

    assert_eq!(check_link(&tiles, &[], 1, 99), Err(SchemeError::NotFound("tile")));
    assert_eq!(check_link(&tiles, &[link(1, 1, 2)], 1, 2), Err(SchemeError::LinkExists));
}

#[test]
fn a_link_that_closes_a_circle_is_refused() {
    let tiles = tiles();
    // 4 is needed by 2, which provides 1, which is needed by 5 ...
    let links = [link(1, 4, 2), link(2, 2, 1), link(3, 1, 5)];

    // ... so 5 cannot provide 4.
    assert_eq!(check_link(&tiles, &links, 5, 4), Err(SchemeError::Cycle));
    // A requirement cannot provide its own input either.
    assert_eq!(check_link(&tiles, &[link(1, 1, 2)], 2, 1), Err(SchemeError::Cycle));
    // Two requirements cannot precede each other, nor one itself.
    assert_eq!(check_link(&tiles, &[link(1, 2, 5)], 5, 2), Err(SchemeError::Cycle));
    assert_eq!(check_link(&tiles, &[], 2, 2), Err(SchemeError::Cycle));
}

#[test]
fn a_tile_needs_a_title_and_keeps_to_its_kind() {
    let mut tile = TileFields::new(TileKind::Requirement, " ");
    assert!(matches!(tile.check(), Err(SchemeError::Invalid("title", _))));

    tile.title = "Protect stored passwords".to_owned();
    assert_eq!(tile.check(), Ok(()));

    tile.external = Some(External { document: "CC Part 3".to_owned(), place: "ALC_FLR.2".to_owned() });
    assert!(matches!(tile.check(), Err(SchemeError::Invalid("external", _))));

    let mut input = TileFields::new(TileKind::Input, "Flaw remediation procedures");
    input.external = tile.external.clone();
    assert_eq!(input.check(), Ok(()));

    input.instructions = "Write it all".to_owned();
    assert!(matches!(input.check(), Err(SchemeError::Invalid("sections", _))));
}

#[test]
fn an_output_has_sections() {
    let mut output = TileFields::new(TileKind::Output, "Report of ADV_FSP");
    assert_eq!(output.sections, OutputSection::ALL);
    assert_eq!(output.check(), Ok(()));

    output.sections.clear();
    assert!(matches!(output.check(), Err(SchemeError::Invalid("sections", _))));
}

#[test]
fn sources_are_checked() {
    let mut tile = TileFields::new(TileKind::Requirement, "R");
    let source = Source { document_id: 1, page: 3, quote: "The TOE shall".to_owned(), found: false };

    tile.sources = vec![Source { page: 0, ..source.clone() }];
    assert!(matches!(tile.check(), Err(SchemeError::Invalid("sources", _))));

    tile.sources = vec![Source { quote: " ".to_owned(), ..source.clone() }];
    assert!(matches!(tile.check(), Err(SchemeError::Invalid("sources", _))));

    tile.sources = vec![source; MAX_SOURCES + 1];
    assert!(matches!(tile.check(), Err(SchemeError::Invalid("sources", _))));
}

#[test]
fn names_of_kinds_states_and_sections_go_both_ways() {
    for kind in TileKind::ALL {
        assert_eq!(TileKind::from_name(kind.name()), Some(kind));
    }

    for state in [ProposalState::Proposed, ProposalState::Accepted] {
        assert_eq!(ProposalState::from_name(state.name()), Some(state));
    }

    for section in OutputSection::ALL {
        assert_eq!(OutputSection::from_name(section.name()), Some(section));
    }
}

#[test]
fn a_project_needs_a_name() {
    let mut project = ProjectFields {
        name: String::new(),
        version: "1.0".to_owned(),
        description: String::new(),
        author: String::new(),
        documents: Vec::new(),
    };

    assert!(matches!(project.check(), Err(SchemeError::Invalid("name", _))));

    project.name = "CC EAL4".to_owned();
    assert_eq!(project.check(), Ok(()));
}

#[test]
fn a_scheme_needs_a_name_and_pages_in_order() {
    let mut scheme =
        SchemeFields { number: "9.1".to_owned(), name: String::new(), pages: None, state: ProposalState::Accepted };

    assert!(matches!(scheme.check(), Err(SchemeError::Invalid("name", _))));

    scheme.name = "Identification and authentication".to_owned();
    assert_eq!(scheme.check(), Ok(()));

    for (first_page, last_page) in [(0, 3), (5, 4)] {
        scheme.pages = Some(PageSpan { document_id: 1, first_page, last_page });
        assert!(matches!(scheme.check(), Err(SchemeError::Invalid("pages", _))), "{first_page}-{last_page}");
    }

    scheme.pages = Some(PageSpan { document_id: 1, first_page: 4, last_page: 4 });
    assert_eq!(scheme.check(), Ok(()));
}
