//! Adding a tile to a scheme of an open test project: checked on the
//! project in memory, its quotes looked for on their pages, written
//! through to the database, then added in memory.

use crate::db::test_projects::create as store;
use crate::db::user::database::UserDb;
use crate::standards;
use crate::standards::quote::is_on_page;

use crate::test_projects::model::{External, Source, Tile, TileFields};
use crate::test_projects::rules::{SchemeError, StoreError};
use crate::test_projects::schemes::read::existing;
use crate::test_projects::workspace::OpenProject;

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

        let text = standards::read::page(db, source.document_id, source.page)?.ok_or_else(|| {
            SchemeError::Invalid("sources", format!("the document has no page {}", source.page))
        })?;

        checked.push(Source {
            found: is_on_page(&source.quote, &text),
            ..source.clone()
        });
    }

    Ok(checked)
}

/// The fields of a tile as the database keeps them (the single-line
/// texts trimmed), with its checked `sources`: what memory holds, so
/// that it stays the same as the database.
pub(super) fn stored(fields: &TileFields, sources: Vec<Source>) -> TileFields {
    TileFields {
        identifier: fields.identifier.trim().to_owned(),
        title: fields.title.trim().to_owned(),
        place: fields.place.trim().to_owned(),
        external: fields.external.as_ref().map(|external| External {
            document: external.document.trim().to_owned(),
            place: external.place.trim().to_owned(),
        }),
        sources,
        ..fields.clone()
    }
}

/// Adds a tile to scheme `scheme` of the open project; returns its
/// number.
pub fn tile(db: &UserDb, open: &mut OpenProject, scheme: i64, fields: &TileFields) -> Result<i64, StoreError> {
    existing(open, scheme)?;
    let sources = checked_sources(db, open, fields)?;

    let id = store::tile(db, scheme, fields, &sources)?;

    open.data_mut().tiles.push(Tile {
        id,
        scheme,
        fields: stored(fields, sources),
    });

    Ok(id)
}
