//! The whole content of a test project that is not open for editing:
//! the tiles and links of an imported project (`exchange/import.rs`),
//! checked by the rules and written in one transaction. (An open
//! project brings back its own versions: `workspace.rs`.)

use crate::db::test_projects::{read as stored, update as store};
use crate::db::user::database::UserDb;

use super::projects::read::existing;
use crate::model::test_projects::rules::{SchemeError, check_link};
use crate::model::test_projects::{Link, Tile, TileKind};
use crate::test_projects::error::StoreError;

/// Replaces the tiles and links of `project` by `tiles` and `links`.
/// Every tile names its scheme, which must be a scheme of the project.
/// The tiles get new numbers; the links follow them. Their sources are
/// stored as they come, with their `found`, which the caller has
/// checked. Returns the new number of every old one.
pub fn replace(db: &UserDb, project: i64, tiles: &[Tile], links: &[Link]) -> Result<Vec<(i64, i64)>, StoreError> {
    // The normal form (model/): trimmed here, stored as it is.
    let tiles: Vec<Tile> = tiles.iter().map(|tile| Tile { fields: tile.fields.normalized(), ..tile.clone() }).collect();
    let tiles = tiles.as_slice();
    let stored_project = existing(db, project)?;
    let schemes: Vec<i64> = stored::schemes(db, project)?.iter().map(|scheme| scheme.id).collect();

    for tile in tiles {
        tile.fields.check()?;

        if !schemes.contains(&tile.scheme) {
            return Err(SchemeError::NotFound("scheme").into());
        }

        if tile.fields.sources.iter().any(|source| !stored_project.fields.documents.contains(&source.document_id)) {
            return Err(SchemeError::Invalid(
                "sources",
                "a source quotes a document the project does not use".to_owned(),
            )
            .into());
        }
    }

    // The rules of the links, checked on the numbers the tiles have
    // now: the new numbers follow them one to one, so the graph is the
    // same. Nothing is written when one link is refused.
    let kinds: Vec<(i64, TileKind)> = tiles.iter().map(|tile| (tile.id, tile.fields.kind)).collect();
    let mut checked: Vec<Link> = Vec::with_capacity(links.len());

    for link in links {
        if !kinds.iter().any(|(id, _)| *id == link.from) || !kinds.iter().any(|(id, _)| *id == link.to) {
            return Err(SchemeError::NotFound("tile").into());
        }

        check_link(&kinds, &checked, link.from, link.to)?;
        checked.push(*link);
    }

    Ok(store::replace_content(db, project, tiles, links)?)
}
