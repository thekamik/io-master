//! Linking two tiles of an open test project, also of two of its
//! schemes, by the rules of `model/test_projects/rules.rs` (checked against all the links
//! of the project in memory); written through, then added in memory.

use crate::db::test_projects::create as store;
use crate::db::user::database::UserDb;

use crate::model::test_projects::rules::{SchemeError, check_link};
use crate::model::test_projects::{Link, TileKind};
use crate::test_projects::error::StoreError;
use crate::test_projects::workspace::OpenProject;

/// Links tile `from` to tile `to`; returns the number of the link.
pub fn link(db: &UserDb, open: &mut OpenProject, from: i64, to: i64) -> Result<i64, StoreError> {
    let data = open.data();
    let kinds: Vec<(i64, TileKind)> = data.tiles.iter().map(|tile| (tile.id, tile.fields.kind)).collect();

    if !kinds.iter().any(|(id, _)| *id == from) || !kinds.iter().any(|(id, _)| *id == to) {
        return Err(SchemeError::Invalid("link", "only tiles of one project can be linked".to_owned()).into());
    }

    check_link(&kinds, &data.links, from, to)?;

    let project = data.project.id;
    let id = store::link(db, project, from, to)?;
    open.data_mut().links.push(Link { id, from, to });

    Ok(id)
}
