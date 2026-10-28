//! A file of OSCAL (JSON) read into the database, and written from it:
//! the document checked against the schema of NIST both ways, a UUID
//! stored once.

use std::path::PathBuf;

use crate::app::error::AppError;
use crate::db::conversion as db_conversion;
use crate::db::error::DbError;
use crate::db::evaluation_support as db_evaluation_support;
use crate::db::oscal as db_oscal;
use crate::db::oscal::error::OscalError;
use crate::db::own_requirements as db_own_requirements;
use crate::db::user::database::UserDb;
use crate::model_old::oscal::{NotOscal, OscalDocument, OscalSummary};

/// The largest file of OSCAL that is read, in bytes (the catalog of NIST
/// SP 800-53 has about 10 MiB).
pub const MAX_FILE: u64 = 64 * 1024 * 1024;

/// Stores the document in the text of a file of OSCAL; returns what
/// identifies it. Refused when it is not OSCAL, breaks the schema of
/// NIST, or is stored already.
pub fn import(db: &UserDb, text: &str) -> Result<OscalSummary, OscalError> {
    let document = match OscalDocument::from_json(text) {
        Ok(document) => document,
        Err(error) => return Err(OscalError::NotOscal(error)),
    };

    if let Some(stored) = db_oscal::read::summary(db, document.uuid())? {
        return Err(OscalError::AlreadyStored(stored.title));
    }

    db_oscal::create::document(db, &document)?;

    Ok(document.summary())
}

/// The text of the file of document `uuid`: its JSON as OSCAL defines
/// it, checked against the schema of NIST, indented for a person.
pub fn export(db: &UserDb, uuid: &str) -> Result<String, OscalError> {
    let Some(document) = db_oscal::read::document(db, uuid)? else {
        return Err(OscalError::NotFound);
    };

    let value = match document.to_json() {
        Ok(value) => value,
        Err(error) => return Err(OscalError::NotOscal(error)),
    };

    match serde_json::to_string_pretty(&value) {
        Ok(text) => Ok(text),
        Err(error) => Err(OscalError::NotOscal(NotOscal::Types(error.to_string()))),
    }
}

/// Deletes document `uuid` with its evaluation support and, for a catalog,
/// its conversion and, for the catalog of the laboratory, the setting that
/// names it; returns whether it existed.
pub fn delete(db: &UserDb, uuid: &str) -> Result<bool, DbError> {
    db_evaluation_support::delete::of_document(db, uuid)?;
    db_conversion::delete::of_catalog(db, uuid)?;
    db_own_requirements::delete::catalog_if(db, uuid)?;
    db_oscal::delete::document(db, uuid)
}

/// The text of the file at `path`, refused when it is larger than
/// [`MAX_FILE`].
///
/// Called by: `oscal_documents_import` (`evaluation_plan.rs`).
/// Calls: `tokio::fs::metadata`, `tokio::fs::read_to_string`.
pub async fn read_file(path: PathBuf) -> Result<String, AppError> {
    let size = match tokio::fs::metadata(&path).await {
        Ok(metadata) => metadata.len(),
        Err(error) => return Err(AppError::Invalid(format!("the file was not read: {error}"))),
    };

    ensure!(size <= MAX_FILE, AppError::Invalid(format!("the file is larger than {} MiB", MAX_FILE / 1024 / 1024)));

    match tokio::fs::read_to_string(&path).await {
        Ok(text) => Ok(text),
        Err(error) => Err(AppError::Invalid(format!("the file was not read: {error}"))),
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests of `file.rs`, on the examples of NIST.

    use super::*;

    use crate::model_old::oscal::OscalModel;
    use crate::test_support::standards::database;

    const CATALOG: &str = include_str!("../../../../oscal/examples/basic-catalog.json");
    const PLAN: &str = include_str!("../../../../oscal/examples/ifa_assessment-plan-example.json");

    #[test]
    fn files_of_nist_are_imported_listed_exported_and_deleted() {
        let test = database();

        let catalog = import(&test.db, CATALOG).unwrap();
        let plan = import(&test.db, PLAN).unwrap();
        assert_eq!(catalog.model, OscalModel::Catalog);
        assert_eq!(plan.model, OscalModel::AssessmentPlan);
        assert_eq!(db_oscal::read::summaries(&test.db).unwrap().len(), 2);

        // The exported file is the document that was imported.
        let exported: serde_json::Value = serde_json::from_str(&export(&test.db, &catalog.uuid).unwrap()).unwrap();
        let mut original: serde_json::Value = serde_json::from_str(CATALOG).unwrap();
        original.as_object_mut().unwrap().remove("$schema");
        assert_eq!(exported, original);

        // The same document twice is refused, with the name of the first.
        assert!(matches!(import(&test.db, CATALOG), Err(OscalError::AlreadyStored(_))));

        assert!(delete(&test.db, &catalog.uuid).unwrap());
        assert!(matches!(export(&test.db, &catalog.uuid), Err(OscalError::NotFound)));
    }

    #[test]
    fn a_file_that_is_not_oscal_is_refused() {
        let test = database();

        assert!(matches!(import(&test.db, "{\"catalog\": {}}"), Err(OscalError::NotOscal(NotOscal::Schema(_)))));
        assert!(matches!(import(&test.db, "%PDF-1.4"), Err(OscalError::NotOscal(NotOscal::NotJson(_)))));
        assert!(db_oscal::read::summaries(&test.db).unwrap().is_empty());
    }

    #[test]
    fn a_name_for_a_file_keeps_only_safe_characters() {
        assert_eq!(crate::model_old::text::safe_file_name(" CC Part 2: SFR / v1.0 ", "oscal"), "CC Part 2_ SFR _ v1.0");
    }
}
