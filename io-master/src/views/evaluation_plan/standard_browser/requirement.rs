//! A requirement of a catalog of a master project, read for the browser of
//! the standards: its text and parts from the catalog, its quotes from the
//! evaluation support.

use crate::app::error::AppError;
use crate::components::part_rows::view as part_rows;
use crate::db_old::evaluation_support as db_evaluation_support;
use crate::db_old::user::database::UserDb;
use crate::model_old::oscal::catalog::Catalog;
use crate::model_old::own_requirement;

use super::super::own_requirement::view::OwnRequirementView;

use super::view::{QuoteView, RequirementView};

/// Control `id` of `catalog` (stored as `catalog_uuid`) as the browser shows
/// it, with its fields to be changed when `own` (the catalog of the
/// laboratory); refused when the catalog has no such control.
///
/// Called by: `standard_browser_get` (`evaluation_plan.rs`).
/// Calls: `Catalog::locate`, `part_rows::rows`,
/// `db_evaluation_support::read::quotes`.
pub fn read(
    db: &UserDb,
    catalog_uuid: &str,
    catalog: &Catalog,
    id: &str,
    in_project: bool,
    own: bool,
) -> Result<RequirementView, AppError> {
    let location = catalog.locate(id)?;
    let control = match &location {
        Some(location) => catalog.control_at(location),
        None => None,
    };

    let Some(control) = control else {
        return Err(AppError::Invalid("the catalog has no such requirement".to_owned()));
    };

    let stored = db_evaluation_support::read::quotes(db, catalog_uuid, Some(id))?;
    let mut quotes = Vec::with_capacity(stored.len());

    for quote in stored {
        quotes.push(QuoteView {
            document_id: quote.source_document_id,
            page: quote.page,
            text: quote.text,
            found: quote.found,
        });
    }

    Ok(RequirementView {
        catalog_uuid: catalog_uuid.to_owned(),
        id: control.id.clone(),
        label: control.label().unwrap_or(&control.id).to_owned(),
        title: control.title.clone(),
        parts: part_rows::rows(&control.parts)?,
        quotes,
        in_project,
        own: if own { Some(OwnRequirementView::new(own_requirement::of_control(control))) } else { None },
    })
}
