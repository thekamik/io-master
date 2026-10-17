//! Adding a tile to a scheme of an open test project: checked on the
//! project in memory, its quotes looked for on their pages, written
//! through to the database, then added in memory.

use crate::business::standards;
use crate::business::standards::quote::is_on_page;
use crate::db::test_projects::create as store;
use crate::db::user::database::UserDb;

use crate::business::test_projects::error::StoreError;
use crate::business::test_projects::schemes::read::existing;
use crate::business::test_projects::workspace::OpenProject;
use crate::model::test_projects::rules::SchemeError;
use crate::model::test_projects::{Source, Tile, TileFields};

/// Checks a tile for the open project, and finds its quotes on their
/// pages: the sources returned carry `found`, set by the program (a
/// passage the AI made up stays visible as such).
pub(super) fn checked_sources(db: &UserDb, open: &OpenProject, fields: &TileFields) -> Result<Vec<Source>, StoreError> {
    fields.check()?;

    let documents = &open.data().project.fields.documents;
    let mut checked = Vec::with_capacity(fields.sources.len());

    for source in &fields.sources {
        if !documents.contains(&source.document_id) {
            return Err(SchemeError::Invalid(
                "sources",
                "a source quotes a document the project does not use; add it to the project first".to_owned(),
            )
            .into());
        }

        let text = standards::read::page(db, source.document_id, source.page)?
            .ok_or_else(|| SchemeError::Invalid("sources", format!("the document has no page {}", source.page)))?;

        checked.push(Source { found: is_on_page(&source.quote, &text), ..source.clone() });
    }

    Ok(checked)
}

/// Adds a tile to scheme `scheme` of the open project; returns its
/// number.
pub fn tile(db: &UserDb, open: &mut OpenProject, scheme: i64, fields: &TileFields) -> Result<i64, StoreError> {
    // The normal form (model/): trimmed here, stored as it is.
    let fields = &fields.normalized();
    existing(open, scheme)?;
    let sources = checked_sources(db, open, fields)?;

    let id = store::tile(db, scheme, fields, &sources)?;

    open.data_mut().tiles.push(Tile { id, scheme, fields: TileFields { sources, ..fields.clone() } });

    Ok(id)
}
