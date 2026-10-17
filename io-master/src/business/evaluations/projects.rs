//! The evaluation projects as a resource (step 29 of `docs/todo.md`,
//! operations named in step 40). An evaluation is made from a test
//! project: its accepted part is copied and frozen, and the first entry
//! of its journal records where it came from.
//!
//! ```text
//! list()            GET     /evaluations
//! get()             GET     /evaluations/{id}       without its structure
//! data()            GET     /evaluations/{id}/data  with its frozen structure
//! from_project()    POST    /evaluations            from a test project of the installation
//! from_exchange()   POST    /evaluations            from the file of a test project
//! delete()          DELETE  /evaluations/{id}       with everything of it; closed in memory
//! ```

use crate::business::evaluations::error::EvaluationError;
use crate::business::evaluations::workspace::EvaluationWorkspace;
use crate::business::standards;
use crate::business::standards::quote::is_on_page;
use crate::business::test_projects::exchange::format::Exchange;
use crate::business::test_projects::projects as test_projects;
use crate::db::error::DbError;
use crate::db::evaluations as db_evaluations;
use crate::db::evaluations::create::FrozenCopy;
use crate::db::evaluations::journal::create::JournalStep;
use crate::db::user::database::UserDb;
use crate::model::evaluations::{Actor, Evaluation, EvaluationData, EvaluationFields, JournalAction, Origin, Skipped};
use crate::model::test_projects::rules::{ProjectError, check_link};
use crate::model::test_projects::{Link, Project, ProjectData, ProposalState, Scheme, Tile, TileKind};
use crate::model::text::{Entity, Field};

/* ---------------------------------------------------------------
 * Read
 * ------------------------------------------------------------- */

/// Every evaluation, by name.
pub fn list(db: &UserDb) -> Result<Vec<Evaluation>, DbError> {
    db_evaluations::read::evaluations(db)
}

/// One evaluation, without its structure; `None` when there is no such.
pub fn get(db: &UserDb, id: i64) -> Result<Option<Evaluation>, DbError> {
    db_evaluations::read::evaluation(db, id)
}

/// An evaluation with its frozen structure; `None` when there is no such.
pub fn data(db: &UserDb, id: i64) -> Result<Option<EvaluationData>, DbError> {
    let Some(evaluation) = get(db, id)? else {
        return Ok(None);
    };

    Ok(Some(EvaluationData {
        evaluation,
        documents: db_evaluations::read::documents(db, id)?,
        schemes: db_evaluations::read::schemes(db, id)?,
        tiles: db_evaluations::read::tiles(db, id)?,
        links: db_evaluations::read::links(db, id)?,
    }))
}

/* ---------------------------------------------------------------
 * Create
 * ------------------------------------------------------------- */

/// The result of an import.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Imported {
    /// The number of the new evaluation.
    pub id: i64,
    /// What was left out because it was still a proposal.
    pub skipped: Skipped,
}

/// Imports the test project `project` of this installation. Only a
/// project marked ready is imported: the person who made it has
/// declared it complete (step 21 of `docs/todo.md`).
pub fn from_project(db: &UserDb, project: i64, fields: &EvaluationFields) -> Result<Imported, EvaluationError> {
    let data = test_projects::data(db, project)?.ok_or(EvaluationError::NotFound(Entity::TestProject))?;

    if !data.project.ready {
        return Err(EvaluationError::Invalid(
            Field::Project,
            "the test project is not marked ready; mark it ready in Create evaluation scheme first".to_owned(),
        ));
    }

    copy(db, &data, fields)
}

/// Imports the test project of a file. `documents` gives the loaded
/// document of every document of the file, in its order (found by its
/// SHA-256, or loaded from the PDF the file carries; done by the
/// caller, which reports the progress). Whether a quote is found is
/// checked again against these copies, as `exchange::store` does.
///
/// A file carries no mark "ready": whoever exported the project gave
/// it out to be carried out.
pub fn from_exchange(
    db: &UserDb,
    exchange: &Exchange,
    documents: &[i64],
    fields: &EvaluationFields,
) -> Result<Imported, EvaluationError> {
    // The documents of the file are numbered 1, 2, ... in its order.
    let document = |number: i64| {
        documents
            .get(usize::try_from(number - 1).map_err(|_| EvaluationError::NotFound(Entity::Document))?)
            .copied()
            .ok_or(EvaluationError::NotFound(Entity::Document))
    };

    let mut schemes = exchange.schemes.clone();

    for scheme in &mut schemes {
        if let Some(pages) = &mut scheme.fields.pages {
            pages.document_id = document(pages.document_id)?;
        }
    }

    let mut tiles = exchange.tiles.clone();

    for tile in &mut tiles {
        for source in &mut tile.fields.sources {
            source.document_id = document(source.document_id)?;
            source.found = standards::documents::page(db, source.document_id, source.page)?
                .is_some_and(|page| is_on_page(&source.quote, &page));
        }
    }

    let mut project_fields = exchange.project.clone();
    project_fields.documents = documents.to_vec();

    let data = ProjectData {
        project: Project { id: 0, fields: project_fields, ready: true },
        schemes,
        tiles,
        links: exchange.links.clone(),
    };

    copy(db, &data, fields)
}

/// Copies the accepted part of `data` into a new evaluation.
fn copy(db: &UserDb, data: &ProjectData, fields: &EvaluationFields) -> Result<Imported, EvaluationError> {
    // The normal form (model/): trimmed here, stored as it is.
    let project = &data.project.fields.normalized();
    let mut skipped = Skipped::default();

    let schemes: Vec<_> = data
        .schemes
        .iter()
        .filter(|scheme| scheme.fields.state == ProposalState::Accepted)
        .map(|scheme| Scheme { fields: scheme.fields.normalized(), ..scheme.clone() })
        .collect();
    skipped.schemes = data.schemes.len() - schemes.len();

    let tiles: Vec<_> = data
        .tiles
        .iter()
        .filter(|tile| {
            tile.fields.state == ProposalState::Accepted && schemes.iter().any(|scheme| scheme.id == tile.scheme)
        })
        .map(|tile| Tile { fields: tile.fields.normalized(), ..tile.clone() })
        .collect();
    skipped.tiles = data.tiles.len() - tiles.len();

    let kept = |id: i64| tiles.iter().any(|tile| tile.id == id);
    let links: Vec<Link> = data.links.iter().filter(|link| kept(link.from) && kept(link.to)).copied().collect();
    skipped.links = data.links.len() - links.len();

    // The rules again: a file may have been changed by hand, and the
    // evaluation is never corrected afterwards.
    for scheme in &schemes {
        scheme.fields.check()?;

        if scheme.fields.pages.is_some_and(|pages| !project.documents.contains(&pages.document_id)) {
            let reason = "a scheme covers pages of a document the project does not use".to_owned();
            return Err(ProjectError::Invalid(Field::Pages, reason).into());
        }
    }

    for tile in &tiles {
        tile.fields.check()?;

        if tile.fields.sources.iter().any(|source| !project.documents.contains(&source.document_id)) {
            let reason = "a source quotes a document the project does not use".to_owned();
            return Err(ProjectError::Invalid(Field::Sources, reason).into());
        }
    }

    let kinds: Vec<(i64, TileKind)> = tiles.iter().map(|tile| (tile.id, tile.fields.kind)).collect();
    let mut checked: Vec<Link> = Vec::with_capacity(links.len());

    for link in &links {
        check_link(&kinds, &checked, link.from, link.to)?;
        checked.push(*link);
    }

    let origin = Origin {
        name: project.name.clone(),
        version: project.version.clone(),
        author: project.author.clone(),
        description: project.description.clone(),
    };

    let detail = format!(
        "Imported the test project \"{}\" (version \"{}\", author \"{}\"): {} schemes, {} tiles, {} links. \
         Left out as proposals never accepted: {} schemes, {} tiles, {} links.",
        origin.name,
        origin.version,
        origin.author,
        schemes.len(),
        tiles.len(),
        links.len(),
        skipped.schemes,
        skipped.tiles,
        skipped.links
    );

    let copy = FrozenCopy { documents: &project.documents, schemes: &schemes, tiles: &tiles, links: &links };

    let id = create(db, fields, &origin, &copy, &detail)?;

    Ok(Imported { id, skipped })
}

/// Writes a new evaluation with its frozen structure and the entry
/// `detail` of its import in the journal; returns its number. The
/// fields are checked here; the structure by [`copy`].
/// On an error nothing of it is left.
fn create(
    db: &UserDb,
    fields: &EvaluationFields,
    origin: &Origin,
    copy: &FrozenCopy<'_>,
    detail: &str,
) -> Result<i64, EvaluationError> {
    // The normal form (model/): trimmed here, stored as it is.
    let fields = &fields.normalized();
    fields.check()?;

    if db_evaluations::read::name_taken(db, &fields.name)? {
        return Err(EvaluationError::NameTaken);
    }

    let step =
        JournalStep { actor: &Actor::Evaluator, action: JournalAction::Imported, tile: None, field: None, detail };

    Ok(db_evaluations::create::evaluation(db, fields, origin, copy, &step)?)
}

/* ---------------------------------------------------------------
 * Delete
 * ------------------------------------------------------------- */

/// Deletes evaluation `id` and closes it; returns whether it existed.
pub fn delete(db: &UserDb, workspace: &mut EvaluationWorkspace, id: i64) -> Result<bool, DbError> {
    let deleted = db_evaluations::delete::evaluation(db, id)?;
    workspace.close(id);

    Ok(deleted)
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "projects_tests.rs"]
mod tests;

// The tests of the import of an evaluation (from a test project or a
// file), in a file of their own.
#[cfg(test)]
#[path = "projects_import_tests.rs"]
mod import_tests;
