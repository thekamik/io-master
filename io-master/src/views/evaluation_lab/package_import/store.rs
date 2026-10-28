//! A work package stored as a new evaluation, all or nothing (step 52 of
//! docs/history.md; made atomic after the audit of 2026-10-25): every check
//! first (the package not imported yet, the catalog of every task and of
//! every related element attached or stored), then the attached catalogs
//! not stored yet and the evaluation; when the evaluation cannot be stored,
//! the catalogs stored by this import are removed again. An attached
//! catalog whose UUID is stored already is not replaced: the stored one is
//! used, and when the two differ the evaluator is told.

use crate::app::error::AppError;
use crate::db::evaluation as db_evaluation;
use crate::db::oscal as db_oscal;
use crate::db::user::database::UserDb;
use crate::model_old::evaluation::{Evaluation, Package};
use crate::model_old::oscal::OscalDocument;

/// Stores `made` with the frozen copy of `package` and the catalogs of
/// `catalogs` not stored yet; gives the notices for the evaluator (an
/// attached catalog that differs from the stored one of its UUID).
///
/// Called by: `package_import_import` (`evaluation_lab.rs`).
/// Calls: [`check`], [`store_catalogs`], `db_evaluation::create::evaluation`, `db_oscal::delete::document`.
pub fn store(
    db: &UserDb,
    made: &Evaluation,
    package: &Package,
    catalogs: &[OscalDocument],
) -> Result<Vec<String>, AppError> {
    check(db, package, catalogs)?;

    let (stored, notices) = store_catalogs(db, catalogs)?;

    if let Err(error) = db_evaluation::create::evaluation(db, made, package) {
        for uuid in &stored {
            // The import fails with its own error; a catalog that cannot be
            // removed again is named in it, so that nothing is left unsaid.
            if let Err(removal) = db_oscal::delete::document(db, uuid) {
                return Err(AppError::Failed(format!(
                    "the evaluation was not stored ({error}), and the catalog {uuid} stored for it could not be removed: {removal}"
                )));
            }
        }

        return Err(error.into());
    }

    Ok(notices)
}

/// Refused when the package is imported already, or a task or a related
/// element points to a catalog neither attached nor stored. Stores nothing.
///
/// Called by: [`store`].
/// Calls: `db_evaluation::read::package_imported`, [`known`].
fn check(db: &UserDb, package: &Package, catalogs: &[OscalDocument]) -> Result<(), AppError> {
    ensure!(
        !db_evaluation::read::package_imported(db, &package.package_uuid)?,
        AppError::Invalid("this work package is imported already".to_owned())
    );

    for task in &package.tasks {
        ensure!(
            known(db, catalogs, &task.task.catalog_uuid)?,
            AppError::Invalid(format!("the catalog of the task {} is neither attached nor stored", task.task.label))
        );

        for element in &task.related {
            ensure!(
                known(db, catalogs, &element.catalog_uuid)?,
                AppError::Invalid(format!("the catalog of {} is neither attached nor stored", element.label))
            );
        }
    }

    Ok(())
}

/// Whether catalog `uuid` is attached to the package or stored.
///
/// Called by: [`check`].
fn known(db: &UserDb, catalogs: &[OscalDocument], uuid: &str) -> Result<bool, AppError> {
    for catalog in catalogs {
        if catalog.uuid() == uuid {
            return Ok(true);
        }
    }

    Ok(db_oscal::read::summary(db, uuid)?.is_some())
}

/// Stores the catalogs of `catalogs` not stored yet; gives their UUIDs (to
/// remove them again if the evaluation fails) and a notice for every
/// attached catalog that differs from the stored one of its UUID. A
/// catalog that cannot be stored removes those stored before it.
///
/// Called by: [`store`].
/// Calls: `db_oscal::read::document`, `db_oscal::create::document`, `db_oscal::delete::document`.
fn store_catalogs(db: &UserDb, catalogs: &[OscalDocument]) -> Result<(Vec<String>, Vec<String>), AppError> {
    let mut stored: Vec<String> = Vec::with_capacity(catalogs.len());
    let mut notices = Vec::new();

    for catalog in catalogs {
        match db_oscal::read::document(db, catalog.uuid())? {
            Some(existing) if existing != *catalog => notices.push(format!(
                "The catalog \"{}\" is stored already in another version; the stored version is used.",
                existing.summary().title
            )),
            Some(_) => {}
            None => {
                if let Err(error) = db_oscal::create::document(db, catalog) {
                    for uuid in &stored {
                        db_oscal::delete::document(db, uuid)?;
                    }

                    return Err(error.into());
                }

                stored.push(catalog.uuid().to_owned());
            }
        }
    }

    Ok((stored, notices))
}

#[cfg(test)]
mod tests {
    //! Unit tests of `store.rs`.

    use super::*;
    use crate::test_support::db::{basic_catalog, lab_evaluation, lab_package};
    use crate::test_support::standards::database;

    #[test]
    fn a_package_whose_catalog_is_missing_stores_nothing() {
        let test = database();
        let catalog = basic_catalog();
        let mut package = lab_package(catalog.uuid());
        package.tasks[0].related = vec![crate::model_old::evaluation::Related {
            catalog_uuid: "no-such-catalog".to_owned(),
            control_id: "x".to_owned(),
            label: "X".to_owned(),
            title: "X".to_owned(),
        }];

        assert!(store(&test.db, &lab_evaluation(&package), &package, std::slice::from_ref(&catalog)).is_err());

        assert!(
            db_oscal::read::summary(&test.db, catalog.uuid()).unwrap().is_none(),
            "the attached catalog is not left"
        );
        assert!(db_evaluation::read::evaluations(&test.db).unwrap().is_empty());
    }

    #[test]
    fn a_catalog_stored_in_another_version_is_kept_and_told() {
        let test = database();
        let catalog = basic_catalog();
        db_oscal::create::document(&test.db, &catalog).unwrap();
        let mut other = catalog.clone();
        if let OscalDocument::Catalog(inner) = &mut other {
            inner.metadata.version = "9.9".to_owned();
        }
        let package = lab_package(catalog.uuid());

        let notices = store(&test.db, &lab_evaluation(&package), &package, &[other]).unwrap();

        assert_eq!(notices.len(), 1);
        assert_eq!(db_oscal::read::summary(&test.db, catalog.uuid()).unwrap().unwrap().version, "1.1");
        assert!(store(&test.db, &lab_evaluation(&package), &package, &[]).is_err(), "imported already");
    }
}
