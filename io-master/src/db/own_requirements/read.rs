//! Reading which catalog is the shared catalog of the requirements of the
//! laboratory (step 51.1e of docs/todo.md). Its UUID is a setting of the
//! evaluator (`db/user/`, table `settings`), so this area has no table of
//! its own; the catalog itself is a document of OSCAL (`db/oscal/`).

use crate::db::error::DbError;
use crate::db::user;
use crate::db::user::database::UserDb;

/// The name of the setting that holds the UUID of the catalog.
pub(in crate::db) const SETTING: &str = "own_requirements.catalog";

/// The UUID of the shared catalog of the requirements of the laboratory;
/// `None` until the first one is written.
///
/// Called by: `views/evaluation_plan/own_requirement/catalog.rs`.
pub fn catalog(db: &UserDb) -> Result<Option<String>, DbError> {
    user::read::setting(db, SETTING)
}
