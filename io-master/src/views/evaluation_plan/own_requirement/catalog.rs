//! The shared catalog of the requirements of the laboratory (step 51.1e of
//! docs/history.md): made at the first requirement, its UUID kept by
//! `db/own_requirements/`; every requirement a control of it, written,
//! changed and removed as the catalogs are (`db/oscal/`, checked against the
//! schema of NIST).

use crate::app::error::AppError;
use crate::db::oscal as db_oscal;
use crate::db::oscal::ids::{new_uuid, now_date_time};
use crate::db::own_requirements as db_own_requirements;
use crate::db::user::database::UserDb;
use crate::model::oscal::OscalDocument;
use crate::model::oscal::catalog::Catalog;
use crate::model::oscal::catalog::edit::Place;
use crate::model::own_requirement::{self, CATALOG_TITLE, OwnRequirement};

/// The UUID of the shared catalog, if it exists (its document may have been
/// deleted: then there is none).
///
/// Called by: [`ensure`], and the browser of the standards (whether a
/// requirement is the laboratory's own).
/// Calls: `db_own_requirements::read::catalog`, `db_oscal::read::summaries`.
pub fn existing(db: &UserDb) -> Result<Option<String>, AppError> {
    let Some(uuid) = db_own_requirements::read::catalog(db)? else {
        return Ok(None);
    };

    for summary in db_oscal::read::summaries(db)? {
        if summary.uuid == uuid {
            return Ok(Some(uuid));
        }
    }

    Ok(None)
}

/// The UUID of the shared catalog, made now when there is none.
///
/// Called by: `own_requirement_add` (`evaluation_plan.rs`).
/// Calls: [`existing`], `Catalog::empty`, `db_oscal::create::document`,
/// `db_own_requirements::update::catalog`.
pub fn ensure(db: &UserDb) -> Result<String, AppError> {
    if let Some(uuid) = existing(db)? {
        return Ok(uuid);
    }

    let document = OscalDocument::Catalog(Catalog::empty(new_uuid()?, CATALOG_TITLE, "", now_date_time()));

    db_oscal::create::document(db, &document)?;
    db_own_requirements::update::catalog(db, document.uuid())?;

    Ok(document.uuid().to_owned())
}

/// Adds `requirement` to the catalog `uuid`, at its top; refused when the
/// catalog has an object of its id already. Gives the id of its control.
///
/// Called by: `own_requirement_add` (`evaluation_plan.rs`).
/// Calls: `own_requirement::control`, `Catalog::add_control`,
/// `db_oscal::update::catalog`.
pub fn add(db: &UserDb, uuid: &str, requirement: &OwnRequirement) -> Result<String, AppError> {
    let control = own_requirement::control(requirement);
    let id = control.id.clone();
    let mut catalog = db_oscal::read::catalog(db, uuid)?;

    catalog.add_control(&Place::Top, control)?;
    db_oscal::update::catalog(db, catalog)?;

    Ok(id)
}

/// Replaces control `id` of the catalog `uuid` by `requirement`, which keeps
/// the id (the tasks point to it).
///
/// Called by: `own_requirement_save` (`evaluation_plan.rs`).
/// Calls: `own_requirement::control`, `Catalog::update_control`,
/// `db_oscal::update::catalog`.
pub fn save(db: &UserDb, uuid: &str, id: &str, requirement: &OwnRequirement) -> Result<(), AppError> {
    let mut control = own_requirement::control(requirement);
    control.id = id.to_owned();

    for part in &mut control.parts {
        part.id = Some(format!("{id}_smt"));
    }

    let mut catalog = db_oscal::read::catalog(db, uuid)?;

    catalog.update_control(id, control)?;
    db_oscal::update::catalog(db, catalog)?;

    Ok(())
}

/// Removes control `id` from the catalog `uuid`.
///
/// Called by: `own_requirement_delete` (`evaluation_plan.rs`), when no task
/// points to it.
/// Calls: `Catalog::remove`, `db_oscal::update::catalog`.
pub fn remove(db: &UserDb, uuid: &str, id: &str) -> Result<(), AppError> {
    let mut catalog = db_oscal::read::catalog(db, uuid)?;

    catalog.remove(id)?;
    db_oscal::update::catalog(db, catalog)?;

    Ok(())
}
