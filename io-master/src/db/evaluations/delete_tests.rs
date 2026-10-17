//! Unit tests of `delete.rs`.

use super::*;

use crate::business::evaluations::projects;
use crate::business::evaluations::test_support::single as results;
use crate::business::evaluations::test_support::{fields, ready_project};
use crate::business::test_projects::test_support::database;
use crate::model::evaluations::ResultField;
use crate::model::test_projects::TileKind;

/// The rows of `table`.
fn rows(db: &UserDb, table: &str) -> i64 {
    db.connection().query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row.get(0)).unwrap()
}

#[test]
fn an_evaluation_leaves_no_row_behind() {
    let test = database();
    let ready = ready_project(&test.db);
    let id = projects::from_project(&test.db, ready.project, &fields("Meter X")).unwrap().id;

    let data = projects::data(&test.db, id).unwrap().unwrap();
    let requirement = data.tiles.iter().find(|tile| tile.fields.kind == TileKind::Requirement).unwrap().id;
    results::write(&test.db, requirement, ResultField::Findings, "None.").unwrap();
    results::propose(&test.db, requirement, ResultField::Analysis, "Proposed.", "gemma4").unwrap();

    assert!(evaluation(&test.db, id).unwrap());

    for table in [
        "evaluations",
        "evaluation_results",
        "evaluation_proposals",
        "evaluation_tile_sources",
        "evaluation_links",
        "evaluation_tiles",
        "evaluation_schemes",
        "evaluation_documents",
        "evaluation_journal",
    ] {
        assert_eq!(rows(&test.db, table), 0, "{table}");
    }

    assert!(!evaluation(&test.db, id).unwrap());
}
