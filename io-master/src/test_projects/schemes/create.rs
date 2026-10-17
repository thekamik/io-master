//! Adding a scheme (a chapter of the standard) to an open test
//! project: checked on the project in memory, written through to the
//! database, then added in memory.

use crate::db::test_projects::create as store;
use crate::db::user::database::UserDb;
use crate::standards;

use crate::test_projects::model::{Scheme, SchemeFields};
use crate::test_projects::rules::{SchemeError, StoreError};
use crate::test_projects::workspace::OpenProject;

/// Checks the fields of a scheme of the open project: its pages in a
/// document of the project, its number not used by another scheme of
/// it (`id`: the scheme itself, when it is changed).
pub(super) fn check(db: &UserDb, open: &OpenProject, id: Option<i64>, fields: &SchemeFields) -> Result<(), StoreError> {
    fields.check()?;

    let data = open.data();

    if let Some(pages) = fields.pages {
        if !data.project.fields.documents.contains(&pages.document_id) {
            return Err(SchemeError::Invalid(
                "pages",
                "the pages of a scheme must be in a document of its project".to_owned(),
            )
            .into());
        }

        let count = standards::read::get(db, pages.document_id)?.map_or(0, |document| document.page_count);

        if pages.last_page > count {
            return Err(SchemeError::Invalid("pages", format!("the document has only {count} pages")).into());
        }
    }

    let number = fields.number.trim();

    if !number.is_empty()
        && data
            .schemes
            .iter()
            .any(|other| Some(other.id) != id && other.fields.number.trim().eq_ignore_ascii_case(number))
    {
        return Err(
            SchemeError::Invalid("number", format!("another scheme of the project has the number {number}")).into()
        );
    }

    Ok(())
}

/// The fields as the database keeps them (the names trimmed).
pub(super) fn stored(fields: &SchemeFields) -> SchemeFields {
    SchemeFields { number: fields.number.trim().to_owned(), name: fields.name.trim().to_owned(), ..fields.clone() }
}

/// Adds a scheme at the end of the list of the open project; returns
/// its number.
pub fn scheme(db: &UserDb, open: &mut OpenProject, fields: &SchemeFields) -> Result<i64, StoreError> {
    check(db, open, None, fields)?;

    let project = open.data().project.id;
    let id = store::scheme(db, project, fields)?;

    open.data_mut().schemes.push(Scheme { id, project, fields: stored(fields) });

    Ok(id)
}
