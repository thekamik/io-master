//! How far a conversion is, read for the page: the stage and the pages of
//! the conversion (`db/conversion/`), the groups with the controls they hold
//! (the catalog, `db/oscal/`), and what is left to check (the evaluation
//! support). Also the check of pages the page or the AI give.

use crate::app::error::AppError;
use crate::db::conversion as db_conversion;
use crate::db::evaluation_support as db_evaluation_support;
use crate::db::knowledge as db_knowledge;
use crate::db::oscal as db_oscal;
use crate::db::standards as db_standards;
use crate::db::user::database::UserDb;
use crate::model::conversion::{self, Pages, Stage};
use crate::model::knowledge::AgentPlace;
use crate::model::oscal::catalog::Catalog;

use super::view::{GroupView, PagesForm, PagesView, ReviewView, WizardView};

/// The wizard of catalog `uuid`, as the page shows it.
///
/// Called by: every command of the wizard (`catalog_editor.rs`).
/// Calls: `db_conversion::read`, `db_oscal::read::catalog`,
/// `db_evaluation_support::read`, [`group_views`].
pub fn read(db: &UserDb, uuid: &str) -> Result<WizardView, AppError> {
    let catalog = db_oscal::read::catalog(db, uuid)?;
    let conversion = db_conversion::read::conversion(db, uuid)?;

    let (stage, contents) = match conversion {
        Some(conversion) => (conversion.stage, conversion.contents),
        // Imported, not converted: finished.
        None => (Stage::Finished, None),
    };

    let groups = group_views(&catalog, db, uuid)?;
    let proposals = db_evaluation_support::read::proposals(db, uuid)?.len();
    let mut quotes_not_found = 0;

    for quote in db_evaluation_support::read::quotes(db, uuid, None)? {
        if !quote.found {
            quotes_not_found += 1;
        }
    }

    let mut groups_without_controls = 0;

    for group in &groups {
        if group.controls == 0 {
            groups_without_controls += 1;
        }
    }

    Ok(WizardView {
        stage: stage.name(),
        contents: contents.map(PagesView::new),
        groups,
        review: ReviewView { proposals, quotes_not_found, groups_without_controls },
        agent: db_knowledge::read::attachment(db, AgentPlace::Catalog, uuid)?,
    })
}

/// The groups of the conversion of `uuid` with their titles and the
/// controls they hold, in the order the conversion keeps.
///
/// Called by: [`read`].
fn group_views(catalog: &Catalog, db: &UserDb, uuid: &str) -> Result<Vec<GroupView>, AppError> {
    let stored = db_conversion::read::groups(db, uuid)?;
    let mut views = Vec::with_capacity(stored.len());

    for group in stored {
        let location = catalog.locate(&group.group_id)?;
        let found = match &location {
            Some(location) => catalog.group_at(&location.groups),
            None => None,
        };

        // A group the evaluator removed from the catalog in the editor.
        let Some(found) = found else {
            continue;
        };

        views.push(GroupView {
            id: group.group_id.clone(),
            title: found.title.clone(),
            pages: group.pages.map(PagesView::new),
            state: group.state.name(),
            controls: found.controls.len(),
        });
    }

    Ok(views)
}

/// The pages `form` of a document that exists, checked against its pages.
///
/// Called by: the commands of the wizard that take pages from the page.
/// Calls: `db_standards::read::document`, `conversion::check_pages`.
pub fn checked_pages(db: &UserDb, form: &PagesForm) -> Result<Pages, AppError> {
    let Some(document) = db_standards::read::document(db, form.document_id)? else {
        return Err(AppError::Invalid("the document of the standard does not exist".to_owned()));
    };

    let pages = Pages { document_id: form.document_id, first: form.first_page, last: form.last_page };

    conversion::check_pages(pages, document.page_count).map_err(|error| AppError::Invalid(error.to_string()))?;

    Ok(pages)
}
