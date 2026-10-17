//! Unit tests of `projects/` (attached to `read.rs`; they cover the
//! whole folder).

use super::*;

use crate::standards;
use crate::standards::model::DocumentError;
use crate::test_projects::model::{PageSpan, Source, TileFields, TileKind};
use crate::test_projects::test_support::single::{
    self as schemes, create_project as create, delete_project as delete, set_ready, update_project as update,
};
use crate::test_projects::test_support::{database, load, project_fields, project_with_scheme, scheme_fields};

#[test]
fn a_project_is_made_listed_changed_and_deleted() {
    let test = database();
    let document = load(&test.db, b"%PDF project");
    let project = create(&test.db, &project_fields("EN 18031-1", vec![document])).unwrap();

    let stored = get(&test.db, project).unwrap().unwrap();
    assert_eq!(stored.fields, project_fields("EN 18031-1", vec![document]));
    assert!(!stored.ready);

    update(&test.db, project, &project_fields("EN 18031-2", vec![document])).unwrap();
    set_ready(&test.db, project, true).unwrap();

    let stored = get(&test.db, project).unwrap().unwrap();
    assert_eq!(stored.fields.name, "EN 18031-2");
    assert!(stored.ready);
    assert_eq!(list(&test.db).unwrap().len(), 1);

    assert!(delete(&test.db, project).unwrap());
    assert!(get(&test.db, project).unwrap().is_none());
}

#[test]
fn names_are_unique_and_documents_must_exist() {
    let test = database();
    let document = load(&test.db, b"%PDF names");
    create(&test.db, &project_fields("ADV and ATE", vec![document])).unwrap();

    assert!(matches!(
        create(&test.db, &project_fields("adv AND ate", vec![document])),
        Err(StoreError::Scheme(SchemeError::NameTaken))
    ));
    assert!(matches!(
        create(&test.db, &project_fields("Other", vec![99])),
        Err(StoreError::Scheme(SchemeError::NotFound("document")))
    ));
}

#[test]
fn the_data_holds_the_schemes_their_tiles_and_the_links() {
    let test = database();
    let document = load(&test.db, b"%PDF data");
    let (project, first) = project_with_scheme(&test.db, document);
    let second = schemes::create(&test.db, project, &scheme_fields("6", "ATE")).unwrap();

    let requirement = schemes::add_tile(&test.db, first, &TileFields::new(TileKind::Requirement, "ADV_FSP.4")).unwrap();
    let later = schemes::add_tile(&test.db, second, &TileFields::new(TileKind::Requirement, "ATE_IND.2")).unwrap();
    schemes::add_link(&test.db, requirement, later).unwrap();

    let data = data(&test.db, project).unwrap().unwrap();

    assert_eq!(data.schemes.iter().map(|scheme| scheme.id).collect::<Vec<_>>(), [first, second]);
    assert_eq!(data.tiles.len(), 2);
    assert_eq!(data.tiles[1].scheme, second);
    assert_eq!(data.links.len(), 1);
}

#[test]
fn a_used_document_stays() {
    let test = database();
    let document = load(&test.db, b"%PDF used");
    let (project, scheme) = project_with_scheme(&test.db, document);

    let mut tile = TileFields::new(TileKind::Requirement, "R");
    tile.sources = vec![Source { document_id: document, page: 1, quote: "Scope".to_owned(), found: false }];
    let id = schemes::add_tile(&test.db, scheme, &tile).unwrap();

    // Not taken from the project while a tile quotes it ...
    assert!(matches!(
        update(&test.db, project, &project_fields("P", vec![])),
        Err(StoreError::Scheme(SchemeError::Invalid("documents", _)))
    ));

    // ... nor while a scheme covers its pages ...
    schemes::delete_tile(&test.db, id).unwrap();
    let mut fields = scheme_fields("5", "ADV");
    fields.pages = Some(PageSpan { document_id: document, first_page: 1, last_page: 2 });
    schemes::update(&test.db, scheme, &fields).unwrap();
    assert!(update(&test.db, project, &project_fields("P", vec![])).is_err());

    // ... and not deleted while a project uses it.
    assert!(matches!(standards::delete::delete(&test.db, document), Err(DocumentError::InUse)));

    delete(&test.db, project).unwrap();
    assert!(standards::delete::delete(&test.db, document).unwrap());
}

#[test]
fn deleting_a_project_takes_everything_of_it() {
    let test = database();
    let document = load(&test.db, b"%PDF delete");
    let (project, scheme) = project_with_scheme(&test.db, document);
    let input = schemes::add_tile(&test.db, scheme, &TileFields::new(TileKind::Input, "A")).unwrap();
    let requirement = schemes::add_tile(&test.db, scheme, &TileFields::new(TileKind::Requirement, "B")).unwrap();
    schemes::add_link(&test.db, input, requirement).unwrap();

    delete(&test.db, project).unwrap();

    assert!(schemes::get(&test.db, scheme).unwrap().is_none());
    assert!(schemes::tile(&test.db, input).unwrap().is_none());
    assert!(schemes::links(&test.db, project).unwrap().is_empty());
}
