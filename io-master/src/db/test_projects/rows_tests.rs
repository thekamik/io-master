//! Unit tests of `rows.rs`: a value the program never writes is an
//! error, never replaced by one that looks right.

use crate::db::error::DbError;
use crate::db::test_projects::read;
use crate::model::test_projects::{OutputSection, TileFields, TileKind};
use crate::test_projects::test_support::single::add_tile;
use crate::test_projects::test_support::{database, load, project_with_scheme};

#[test]
fn a_tile_changed_outside_the_program_is_an_error() {
    let test = database();
    let document = load(&test.db, b"%PDF rows");
    let (_, scheme) = project_with_scheme(&test.db, document);
    let mut output = TileFields::new(TileKind::Output, "Report");
    output.sections = vec![OutputSection::Requirements];
    let id = add_tile(&test.db, scheme, &output).unwrap();
    assert!(read::tile(&test.db, id).unwrap().is_some());

    for (column, value) in [("kind", "gadget"), ("state", "approved"), ("sections", "requirements,gossip")] {
        test.db
            .connection_for_tests()
            .execute(&format!("UPDATE tiles SET kind = 'output', state = 'accepted', {column} = ?1"), [value])
            .unwrap();

        let error = read::tile(&test.db, id).unwrap_err();
        assert!(
            matches!(&error, DbError::Corrupt(corrupt) if corrupt.table == "tiles" && corrupt.column == column),
            "{column}: {error}"
        );
    }
}
