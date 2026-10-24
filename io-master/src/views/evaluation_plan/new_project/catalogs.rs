//! The catalogs a master project can use: the finished ones (converted to
//! their end in the view Converter, or imported), and the check of the
//! catalogs chosen for a project. Shared by the new project and the form of
//! a project, which owns no data of its own about catalogs.

use crate::app::error::AppError;
use crate::db::conversion as db_conversion;
use crate::db::oscal as db_oscal;
use crate::db::user::database::UserDb;
use crate::model::evaluation_project;
use crate::model::oscal::OscalModel;

use super::view::CatalogChoiceView;

/// The finished catalogs, in the order of the documents of OSCAL.
///
/// Called by: `new_project_catalogs`, `project_form_get` (`evaluation_plan.rs`),
/// and [`check`].
/// Calls: `db_oscal::read::summaries`, `db_conversion::read::finished`.
pub fn finished(db: &UserDb) -> Result<Vec<CatalogChoiceView>, AppError> {
    let summaries = db_oscal::read::summaries(db)?;
    let mut choices = Vec::with_capacity(summaries.len());

    for summary in &summaries {
        if summary.model == OscalModel::Catalog && db_conversion::read::finished(db, &summary.uuid)? {
            choices.push(CatalogChoiceView::new(summary));
        }
    }

    Ok(choices)
}

/// Checks the catalogs chosen for a project: the rules of the model (some,
/// none twice), and each one a finished catalog.
///
/// Called by: `new_project_create`, `project_form_save` (`evaluation_plan.rs`).
/// Calls: `evaluation_project::check_catalogs`, [`finished`].
pub fn check(db: &UserDb, chosen: &[String]) -> Result<(), AppError> {
    evaluation_project::check_catalogs(chosen).map_err(|error| AppError::Invalid(error.to_string()))?;

    let finished = finished(db)?;

    for catalog in chosen {
        let mut found = false;

        for choice in &finished {
            if &choice.uuid == catalog {
                found = true;
            }
        }

        ensure!(found, AppError::Invalid("a project uses only finished catalogs".to_owned()));
    }

    Ok(())
}
