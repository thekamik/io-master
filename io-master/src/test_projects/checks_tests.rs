//! Unit tests of `checks.rs`.

use super::*;

use crate::test_projects::model::{External, Source, TileFields};

fn tile(id: i64, kind: TileKind) -> Tile {
    let mut fields = TileFields::new(kind, "T");

    if kind == TileKind::Requirement {
        fields.sources = vec![Source {
            document_id: 1,
            page: 1,
            quote: "q".to_owned(),
            found: true,
        }];
    }

    Tile { id, scheme: 1, fields }
}

fn link(from: i64, to: i64) -> Link {
    Link { id: from * 100 + to, from, to }
}

fn kinds(findings: &[Finding], tile: i64) -> Vec<FindingKind> {
    findings.iter().filter(|finding| finding.tile == tile).map(|finding| finding.kind).collect()
}

#[test]
fn a_complete_chain_has_no_findings() {
    let tiles = [tile(1, TileKind::Input), tile(2, TileKind::Requirement), tile(3, TileKind::Output)];

    assert!(check(&tiles, &[link(1, 2), link(2, 3)]).is_empty());
}

#[test]
fn a_preceding_requirement_counts_as_an_input() {
    let tiles = [tile(1, TileKind::Requirement), tile(2, TileKind::Requirement), tile(3, TileKind::Output)];
    let findings = check(&tiles, &[link(1, 2), link(2, 3), link(1, 3)]);

    // 2 needs 1 met first; 1 itself still has nothing before it.
    assert_eq!(kinds(&findings, 2), []);
    assert_eq!(kinds(&findings, 1), [FindingKind::RequirementWithoutInputs]);
}

#[test]
fn the_gaps_are_found() {
    let tiles = [tile(1, TileKind::Input), tile(2, TileKind::Requirement), tile(3, TileKind::Output)];
    let findings = check(&tiles, &[]);

    assert_eq!(kinds(&findings, 1), [FindingKind::UnusedInput]);
    assert_eq!(
        kinds(&findings, 2),
        [FindingKind::RequirementWithoutInputs, FindingKind::RequirementNotReported]
    );
    assert_eq!(kinds(&findings, 3), [FindingKind::EmptyOutput]);
}

#[test]
fn proposed_external_untraced_and_unfound_are_found() {
    let mut external = tile(1, TileKind::Input);
    external.fields.external = Some(External {
        document: "CC Part 3".to_owned(),
        place: "ALC_FLR".to_owned(),
    });

    let mut requirement = tile(2, TileKind::Requirement);
    requirement.fields.state = ProposalState::Proposed;
    requirement.fields.sources[0].found = false;

    let mut untraced = tile(3, TileKind::Requirement);
    untraced.fields.sources.clear();

    let tiles = [external, requirement, untraced, tile(4, TileKind::Output)];
    let findings = check(&tiles, &[link(1, 2), link(2, 4), link(1, 3), link(3, 4)]);

    assert_eq!(kinds(&findings, 1), [FindingKind::InputToComplete]);
    assert_eq!(kinds(&findings, 2), [FindingKind::Proposed, FindingKind::QuoteNotFound]);
    assert_eq!(kinds(&findings, 3), [FindingKind::RequirementWithoutSource]);
    assert!(kinds(&findings, 4).is_empty());
}

#[test]
fn a_provided_input_counts_as_used_only_when_needed() {
    // 2 provides 1, but nothing needs 1.
    let tiles = [tile(1, TileKind::Input), tile(2, TileKind::Requirement)];

    assert!(kinds(&check(&tiles, &[link(2, 1)]), 1).contains(&FindingKind::UnusedInput));
}
