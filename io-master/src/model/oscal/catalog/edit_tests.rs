//! Unit tests of `edit.rs` and `tree.rs`: the changes of a catalog in
//! memory, and the walk through it.

use super::*;

use crate::model::oscal::OscalDocument;

/// The catalog of NIST in the examples, without groups and controls of
/// its own.
fn empty() -> Catalog {
    let OscalDocument::Catalog(mut catalog) =
        OscalDocument::from_json(include_str!("../../../../oscal/examples/basic-catalog.json")).unwrap()
    else {
        panic!("the example is a catalog");
    };

    catalog.groups.clear();
    catalog.controls.clear();
    catalog
}

fn group(id: &str) -> Group {
    serde_json::from_value(serde_json::json!({ "id": id, "title": format!("Group {id}") })).unwrap()
}

fn control(id: &str) -> Control {
    serde_json::from_value(serde_json::json!({ "id": id, "title": format!("Control {id}") })).unwrap()
}

/// ADV with ADV_FSP (two enhancements) and the group ADV_X; ATE at the top.
fn sample() -> Catalog {
    let mut catalog = empty();

    catalog.add_group(&Place::Top, group("ADV")).unwrap();
    catalog.add_control(&Place::Group("ADV".into()), control("ADV_FSP.1")).unwrap();
    catalog.add_control(&Place::Control("ADV_FSP.1".into()), control("ADV_FSP.1.a")).unwrap();
    catalog.add_control(&Place::Control("ADV_FSP.1.a".into()), control("ADV_FSP.1.a.i")).unwrap();
    catalog.add_group(&Place::Group("ADV".into()), group("ADV_X")).unwrap();
    catalog.add_control(&Place::Top, control("ATE")).unwrap();

    catalog
}

#[test]
fn the_walk_gives_the_objects_in_the_order_of_the_document() {
    let catalog = sample();
    let mut ids = Vec::new();

    for location in catalog.locations().unwrap() {
        ids.push(catalog.id_at(&location).unwrap().to_owned());
    }

    assert_eq!(ids, ["ATE", "ADV", "ADV_FSP.1", "ADV_FSP.1.a", "ADV_FSP.1.a.i", "ADV_X"]);
}

#[test]
fn every_control_is_listed_with_the_titles_of_its_groups() {
    let catalog = sample();
    let all = catalog.all_controls().unwrap();

    assert_eq!(all.len(), 4);
    assert_eq!(all[0].0, Vec::<&str>::new());
    assert_eq!(all[2].0, ["Group ADV"]);
    assert_eq!(all[3].1.id, "ADV_FSP.1.a.i");
    assert_eq!(
        catalog.group_titles().unwrap(),
        [("ADV".to_owned(), "Group ADV".to_owned()), ("ADV_X".to_owned(), "Group ADV_X".to_owned())]
    );
}

#[test]
fn an_id_is_used_once_and_a_group_goes_only_into_a_group() {
    let mut catalog = sample();

    assert_eq!(catalog.add_control(&Place::Top, control("ADV")), Err(CatalogError::IdTaken("ADV".into())));
    assert_eq!(catalog.add_group(&Place::Top, group("")), Err(CatalogError::Invalid("the object needs an id".into())));
    assert!(matches!(catalog.add_group(&Place::Control("ATE".into()), group("G")), Err(CatalogError::Invalid(_))));
    assert_eq!(
        catalog.add_control(&Place::Group("ATE".into()), control("C")),
        Err(CatalogError::NoSuchObject("ATE".into()))
    );
}

#[test]
fn a_change_keeps_what_the_object_holds() {
    let mut catalog = sample();

    catalog.update_group("ADV", group("ADV2")).unwrap();
    catalog.update_control("ADV_FSP.1", control("ADV_FSP.2")).unwrap();

    let ids = catalog.ids().unwrap();
    assert!(ids.contains("ADV2") && ids.contains("ADV_FSP.2") && ids.contains("ADV_FSP.1.a.i"));
    assert!(!ids.contains("ADV") && !ids.contains("ADV_FSP.1"));
    assert_eq!(catalog.update_control("ADV_FSP.2", control("ATE")), Err(CatalogError::IdTaken("ATE".into())));
}

#[test]
fn an_object_is_removed_with_everything_it_holds() {
    let mut catalog = sample();

    let gone = catalog.remove("ADV_FSP.1.a").unwrap();
    assert_eq!(gone, ["ADV_FSP.1.a", "ADV_FSP.1.a.i"]);

    let gone = catalog.remove("ADV").unwrap();
    assert_eq!(gone, ["ADV", "ADV_FSP.1", "ADV_X"]);

    assert_eq!(catalog.ids().unwrap().into_iter().collect::<Vec<_>>(), ["ATE"]);
    assert_eq!(catalog.remove("ADV"), Err(CatalogError::NoSuchObject("ADV".into())));
}

#[test]
fn a_catalog_deeper_than_the_limit_is_refused() {
    let mut catalog = empty();
    let mut place = Place::Top;

    for level in 0..=crate::model::oscal::catalog::tree::MAX_DEPTH {
        let id = format!("C{level}");
        catalog.add_control(&place, control(&id)).unwrap_or(());
        place = Place::Control(id);
    }

    assert_eq!(catalog.locations(), Err(CatalogError::TooDeep));
}
