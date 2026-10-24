//! The search of the requirements of the catalogs of a master project: a
//! text looked for, letter case aside, in the identifier, the title and the
//! text of the parts of every control.

use crate::app::error::AppError;
use crate::model::oscal::catalog::Catalog;
use crate::model::oscal::catalog::tree::flat_parts;

use super::view::HitView;

/// The longest text searched for.
pub const MAX_SEARCH: usize = 200;

/// The most hits given back.
pub const MAX_HITS: usize = 200;

/// The controls of `catalogs` (UUID and catalog) that hold `text`, in the
/// order of the catalogs and of their documents; at most [`MAX_HITS`].
///
/// Called by: `standard_browser_search` (`evaluation_plan.rs`).
/// Calls: `Catalog::all_controls`, `flat_parts`, [`holds`].
pub fn search(catalogs: &[(String, Catalog)], text: &str) -> Result<Vec<HitView>, AppError> {
    let wanted = text.trim().to_lowercase();

    ensure!(!wanted.is_empty(), AppError::Invalid("search for a text".to_owned()));
    ensure!(
        wanted.chars().count() <= MAX_SEARCH,
        AppError::Invalid(format!("search for at most {MAX_SEARCH} characters"))
    );

    let mut hits = Vec::new();

    for (uuid, catalog) in catalogs {
        for (_, control) in catalog.all_controls()? {
            if hits.len() >= MAX_HITS {
                return Ok(hits);
            }

            let label = control.label().unwrap_or(&control.id).to_owned();
            let mut found = holds(&label, &wanted) || holds(&control.title, &wanted);

            for (_, part) in flat_parts(&control.parts)? {
                if let Some(prose) = &part.prose
                    && holds(prose, &wanted)
                {
                    found = true;
                }
            }

            if found {
                hits.push(HitView {
                    catalog_uuid: uuid.clone(),
                    catalog_title: catalog.metadata.title.clone(),
                    id: control.id.clone(),
                    label,
                    title: control.title.clone(),
                });
            }
        }
    }

    Ok(hits)
}

/// Whether `text` holds `wanted` (in lower case already), letter case aside.
///
/// Called by: [`search`].
fn holds(text: &str, wanted: &str) -> bool {
    text.to_lowercase().contains(wanted)
}
