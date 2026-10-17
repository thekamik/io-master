//! Help for the tests of the evaluation projects: a ready test project
//! on the small standard of `evaluation/test_support.rs`, and the
//! fields of an evaluation.

use crate::db::user::database::UserDb;
use crate::evaluation::projects as test_projects;
use crate::evaluation::scheme::{Source, TileFields, TileKind};
use crate::evaluation::schemes;
use crate::evaluation::test_support::{self as standard, PAGES};

use super::model::EvaluationFields;

/// The tiles of [`ready_project`].
pub struct ReadyProject {
    pub document: i64,
    pub project: i64,
    pub scheme: i64,
    pub input: i64,
    pub requirement: i64,
    pub output: i64,
}

/// A ready test project "P" on the small standard: one scheme with an
/// input, a requirement quoting page 2 and an output, linked
/// input → requirement → output.
pub fn ready_project(db: &UserDb) -> ReadyProject {
    let document = standard::load(db, b"standard");
    let (project, scheme) = standard::project_with_scheme(db, document);

    let input = schemes::add_tile(db, scheme, &TileFields::new(TileKind::Input, "Functional specification")).unwrap();

    let mut fields = TileFields::new(TileKind::Requirement, "Complete functional specification");
    fields.identifier = "ADV_FSP.4.1E".to_owned();
    fields.sources = vec![Source {
        document_id: document,
        page: 2,
        quote: "The evaluator shall confirm that the information provided meets all requirements.".to_owned(),
        found: false,
    }];
    let requirement = schemes::add_tile(db, scheme, &fields).unwrap();

    let output = schemes::add_tile(db, scheme, &TileFields::new(TileKind::Output, "ADV report")).unwrap();

    schemes::add_link(db, input, requirement).unwrap();
    schemes::add_link(db, requirement, output).unwrap();
    test_projects::set_ready(db, project, true).unwrap();

    assert!(PAGES[1].contains("ADV_FSP.4.1E"));

    ReadyProject {
        document,
        project,
        scheme,
        input,
        requirement,
        output,
    }
}

/// The fields of an evaluation named `name`.
pub fn fields(name: &str) -> EvaluationFields {
    EvaluationFields {
        name: name.to_owned(),
        product: "Smart meter X".to_owned(),
        product_version: "2.1".to_owned(),
        manufacturer: "ACME".to_owned(),
        evaluator: "Kamil".to_owned(),
    }
}
