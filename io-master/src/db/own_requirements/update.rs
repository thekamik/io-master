//! Storing which catalog is the shared catalog of the requirements of the
//! laboratory.

use crate::db::error::DbError;
use crate::db::user;
use crate::db::user::database::UserDb;

use super::read::SETTING;

/// Keeps `uuid` as the shared catalog of the requirements of the laboratory.
///
/// Called by: `views/evaluation_plan/own_requirement/catalog.rs`, when the
/// catalog is made.
pub fn catalog(db: &UserDb, uuid: &str) -> Result<(), DbError> {
    user::update::set_setting(db, SETTING, uuid)
}

#[cfg(test)]
mod tests {
    //! Tests of storing the catalog of the laboratory.

    use super::*;

    use crate::db::own_requirements::{delete, read};
    use crate::test_support::standards::database;

    #[test]
    fn the_catalog_is_kept_and_forgotten_with_its_document() {
        let test = database();

        assert_eq!(read::catalog(&test.db).unwrap(), None);
        catalog(&test.db, "lab").unwrap();
        assert_eq!(read::catalog(&test.db).unwrap().as_deref(), Some("lab"));

        delete::catalog_if(&test.db, "other").unwrap();
        assert!(read::catalog(&test.db).unwrap().is_some());
        delete::catalog_if(&test.db, "lab").unwrap();
        assert_eq!(read::catalog(&test.db).unwrap(), None);
    }
}
