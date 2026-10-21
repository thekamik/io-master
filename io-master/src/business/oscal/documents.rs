//! The documents of OSCAL as the work on catalogs reads them (moved with
//! the editor of a catalog in step 42); the import, the export and the
//! deletion are in `views/plans/oscal_documents/`.
//!
//! ```text
//! get()      GET     /oscal/{uuid}
//! ```

use crate::db::error::DbError;
use crate::db::oscal as db_oscal;
use crate::db::user::database::UserDb;
use crate::model::oscal::OscalDocument;

/// Document `uuid`; `None` when there is no such.
pub fn get(db: &UserDb, uuid: &str) -> Result<Option<OscalDocument>, DbError> {
    db_oscal::read::document(db, uuid)
}
