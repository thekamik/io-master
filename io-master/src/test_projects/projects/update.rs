//! Changing an open test project: its fields and documents, and the
//! mark "ready". Written through to the database, then made in memory.

use crate::db::test_projects::update as store;
use crate::db::user::database::UserDb;

use crate::model::test_projects::ProjectFields;
use crate::model::test_projects::rules::SchemeError;
use crate::test_projects::error::StoreError;
use crate::test_projects::workspace::OpenProject;

use super::create::check;

/// Changes the fields of the open project. A document still quoted by
/// a tile, or holding the pages of a scheme, cannot be taken away.
pub fn fields(db: &UserDb, open: &mut OpenProject, fields: &ProjectFields) -> Result<(), StoreError> {
    // The normal form (model/): trimmed here, stored as it is.
    let fields = &fields.normalized();
    let id = open.data().project.id;
    check(db, Some(id), fields)?;

    let data = open.data();
    let quoted = data.tiles.iter().flat_map(|tile| tile.fields.sources.iter().map(|source| source.document_id));
    let paged = data.schemes.iter().filter_map(|scheme| scheme.fields.pages.map(|pages| pages.document_id));
    let used: Vec<i64> = quoted.chain(paged).collect();

    if used.iter().any(|document| !fields.documents.contains(document)) {
        return Err(SchemeError::Invalid(
            "documents",
            "a tile quotes, or a scheme covers pages of, a document taken away from the project; change those first"
                .to_owned(),
        )
        .into());
    }

    store::project(db, id, fields)?;

    let project = &mut open.data_mut().project;
    project.fields = fields.clone();

    Ok(())
}

/// Marks the open project ready, or a draft again.
pub fn ready(db: &UserDb, open: &mut OpenProject, ready: bool) -> Result<(), StoreError> {
    store::ready(db, open.data().project.id, ready)?;
    open.data_mut().project.ready = ready;

    Ok(())
}
