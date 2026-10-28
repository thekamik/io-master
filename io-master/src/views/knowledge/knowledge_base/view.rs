//! The data of the screen of a knowledge base, as the page gets it.

use serde::Serialize;

use crate::app::error::AppError;
use crate::db_old::knowledge as db_knowledge;
use crate::db_old::oscal as db_oscal;
use crate::db_old::user::database::UserDb;
use crate::model_old::oscal::OscalModel;

/// A source of a base.
#[derive(Debug, Serialize)]
pub struct SourceView {
    pub id: i64,
    /// `pdf`, `text`, `catalog` or `note` (`model/knowledge.rs`, `SourceKind`).
    pub kind: &'static str,
    pub name: String,
    pub passages: usize,
}

/// The screen of a base: its name, its description and its sources.
#[derive(Debug, Serialize)]
pub struct BaseScreenView {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub sources: Vec<SourceView>,
}

/// A catalog of OSCAL of the installation, to be given to a base.
#[derive(Debug, Serialize)]
pub struct CatalogChoiceView {
    pub uuid: String,
    pub title: String,
    pub version: String,
}

/// The screen of base `id`; refused when it does not exist.
///
/// Called by: `knowledge_base_get` and every command that changes a base (`knowledge.rs`).
/// Calls: `db_knowledge::read::base`, `db_knowledge::read::sources`.
pub fn screen(db: &UserDb, id: i64) -> Result<BaseScreenView, AppError> {
    let Some(base) = db_knowledge::read::base(db, id)? else {
        return Err(AppError::Invalid("the knowledge base does not exist".to_owned()));
    };
    let stored = db_knowledge::read::sources(db, id)?;
    let mut sources = Vec::with_capacity(stored.len());

    for source in stored {
        sources.push(SourceView {
            id: source.id,
            kind: source.kind.name(),
            name: source.name,
            passages: source.passages,
        });
    }

    Ok(BaseScreenView { id: base.id, name: base.name, description: base.description, sources })
}

/// The catalogs of OSCAL of the installation, by title.
///
/// Called by: `knowledge_base_catalogs` (`knowledge.rs`).
/// Calls: `db_oscal::read::summaries`.
pub fn catalogs(db: &UserDb) -> Result<Vec<CatalogChoiceView>, AppError> {
    let mut found = Vec::new();

    for summary in db_oscal::read::summaries(db)? {
        if summary.model == OscalModel::Catalog {
            found.push(CatalogChoiceView { uuid: summary.uuid, title: summary.title, version: summary.version });
        }
    }

    Ok(found)
}
