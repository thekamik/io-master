//! Unit tests of `exchange/` (attached to `import.rs`; they cover the
//! whole folder).

use super::*;

use crate::business::test_projects::exchange::export::export;
use crate::business::test_projects::projects::read::{data, list};
use crate::business::test_projects::test_support::single::{add_link, add_tile, create, create_project};
use crate::business::test_projects::test_support::{TestDb, database, load, project_fields};

/// A database with the small standard and a project "ADV" of two
/// schemes: three linked tiles in the first (one traced to page 2),
/// and a requirement in the second that the first one precedes.
/// Returns the project and the document.
fn with_project() -> (TestDb, i64, i64) {
    let test = database();
    let document = load(&test.db, b"%PDF exchange");

    let mut fields = project_fields("ADV", vec![document]);
    fields.version = "2.0".to_owned();
    fields.author = "Head".to_owned();
    let project = create_project(&test.db, &fields).unwrap();

    let first = create(
        &test.db,
        project,
        &SchemeFields {
            number: "5.1".to_owned(),
            name: "Functional specification".to_owned(),
            pages: Some(PageSpan { document_id: document, first_page: 2, last_page: 2 }),
            state: ProposalState::Accepted,
        },
    )
    .unwrap();
    let second = create(
        &test.db,
        project,
        &SchemeFields {
            number: "5.2".to_owned(),
            name: "Testing".to_owned(),
            pages: None,
            state: ProposalState::Proposed,
        },
    )
    .unwrap();

    let input = add_tile(&test.db, first, &TileFields::new(TileKind::Input, "Functional specification")).unwrap();
    let mut traced = TileFields::new(TileKind::Requirement, "ADV_FSP.4");
    traced.sources = vec![Source {
        document_id: document,
        page: 2,
        quote: "The developer shall provide a functional specification.".to_owned(),
        found: false,
    }];
    traced.x = 300.0;
    let requirement = add_tile(&test.db, first, &traced).unwrap();
    let output = add_tile(&test.db, first, &TileFields::new(TileKind::Output, "Report")).unwrap();
    let later = add_tile(&test.db, second, &TileFields::new(TileKind::Requirement, "ATE_IND.2")).unwrap();

    add_link(&test.db, input, requirement).unwrap();
    add_link(&test.db, requirement, output).unwrap();
    add_link(&test.db, requirement, later).unwrap();

    (test, project, document)
}

#[test]
fn a_project_is_exported_and_imported_on_another_installation_with_its_pdf() {
    let (source, project, _) = with_project();
    let file = export(&source.db, project, true).unwrap().unwrap();

    let exchange = read(&file).unwrap();
    assert_eq!(exchange.project.name, "ADV");
    assert_eq!(exchange.documents[0].pdf.as_deref(), Some(&b"%PDF exchange"[..]));
    assert_eq!(exchange.schemes.len(), 2);
    assert_eq!(exchange.tiles.len(), 4);
    assert_eq!(exchange.links.len(), 3);

    // The other installation has its own copy (loaded the same way).
    let other = database();
    let copy = load(&other.db, b"%PDF exchange");

    let imported = store(&other.db, &exchange, &[copy]).unwrap();
    let stored = data(&other.db, imported).unwrap().unwrap();

    assert_eq!(stored.project.fields.author, "Head");
    assert_eq!(stored.schemes.len(), 2);
    assert_eq!(stored.schemes[0].fields.number, "5.1");
    assert_eq!(stored.schemes[0].fields.pages.unwrap().document_id, copy);
    assert_eq!(stored.schemes[1].fields.state, ProposalState::Proposed);

    assert_eq!(stored.tiles.len(), 4);
    assert_eq!(stored.tiles[1].fields.x, 300.0);
    assert!(stored.tiles[1].fields.sources[0].found);
    assert_eq!(stored.tiles[1].fields.sources[0].document_id, copy);
    assert_eq!(stored.tiles[3].scheme, stored.schemes[1].id);
    assert_eq!(stored.links.len(), 3);
}

#[test]
fn without_pdfs_the_documents_are_only_named() {
    let (source, project, document) = with_project();
    let file = export(&source.db, project, false).unwrap().unwrap();
    let exchange = read(&file).unwrap();

    let named = &exchange.documents[0];
    assert!(named.pdf.is_none());
    assert_eq!(named.sha256, standards::read::get(&source.db, document).unwrap().unwrap().sha256);
    assert_eq!(named.page_count, 3);
}

#[test]
fn a_taken_name_gets_a_suffix() {
    let (test, project, document) = with_project();
    let exchange = read(&export(&test.db, project, false).unwrap().unwrap()).unwrap();

    let first = store(&test.db, &exchange, &[document]).unwrap();
    let second = store(&test.db, &exchange, &[document]).unwrap();

    assert_eq!(data(&test.db, first).unwrap().unwrap().project.fields.name, "ADV (imported)");
    assert_eq!(data(&test.db, second).unwrap().unwrap().project.fields.name, "ADV (imported 2)");
}

#[test]
fn quotes_are_checked_against_the_copy_of_the_importer() {
    let (source, project, _) = with_project();
    let exchange = read(&export(&source.db, project, false).unwrap().unwrap()).unwrap();

    // Another document stands in: the quote is not on its page 2.
    let other = database();
    let pages = vec!["a".to_owned(), "b".to_owned()];
    let different = standards::create::add(
        &other.db,
        &crate::business::test_projects::test_support::fields(),
        "x.pdf",
        b"%PDF different",
        &pages,
    )
    .unwrap();

    let imported = store(&other.db, &exchange, &[different]).unwrap();

    assert!(!data(&other.db, imported).unwrap().unwrap().tiles[1].fields.sources[0].found);
}

#[test]
fn broken_files_are_refused() {
    assert!(matches!(read(b"not json"), Err(ExchangeError::Malformed(_))));
    assert!(matches!(read(br#"{"format": "other", "version": 1}"#), Err(ExchangeError::Malformed(_))));
    assert_eq!(
        read(br#"{"format": "io-master-project", "version": 9, "project": {}}"#),
        Err(ExchangeError::NewerVersion(9))
    );

    let unknown_tile = br#"{"format": "io-master-project", "version": 1, "project": {"name": "X"},
        "schemes": [{"key": "s1", "name": "S"}],
        "tiles": [{"key": "t1", "scheme": "s1", "kind": "input", "title": "A"}], "links": [{"from": "t1", "to": "t9"}]}"#;
    assert!(matches!(read(unknown_tile), Err(ExchangeError::Malformed(_))));

    let unknown_scheme = br#"{"format": "io-master-project", "version": 1, "project": {"name": "X"},
        "tiles": [{"key": "t1", "scheme": "s9", "kind": "input", "title": "A"}]}"#;
    assert!(matches!(read(unknown_scheme), Err(ExchangeError::Malformed(_))));

    let wrong_pdf = format!(
        r#"{{"format": "io-master-project", "version": 1, "project": {{"name": "X"}},
        "documents": [{{"key": "d1", "sha256": "00", "pdf": "{}"}}]}}"#,
        BASE64.encode(b"%PDF")
    );
    assert_eq!(read(wrong_pdf.as_bytes()), Err(ExchangeError::WrongPdf("d1".to_owned())));
}

#[test]
fn a_file_that_breaks_the_rules_leaves_nothing() {
    let test = database();
    let file = br#"{"format": "io-master-project", "version": 1, "project": {"name": "Broken"},
        "schemes": [{"key": "s1", "number": "1", "name": "S", "state": "accepted"}],
        "tiles": [{"key": "t1", "scheme": "s1", "kind": "input", "title": "A"},
                  {"key": "t2", "scheme": "s1", "kind": "output", "title": "B", "sections": ["verdict"]}],
        "links": [{"from": "t1", "to": "t2"}]}"#;

    let exchange = read(file).unwrap();

    assert!(store(&test.db, &exchange, &[]).is_err());
    assert!(list(&test.db).unwrap().is_empty());
}
