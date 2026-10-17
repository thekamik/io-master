//! Making an evaluation project: the import of a test project into a
//! new evaluation project
//! (user, 2026-10-17: the evaluation starts when a project is
//! imported; the project is transformed into an evaluation project, a
//! copy).
//!
//! ```text
//! from_project()   a test project of this installation, marked ready
//! from_exchange()  the file of an exported test project
//!                  (test_projects/exchange/), its documents already
//!                  found or loaded
//!        │
//!        ▼  both as one ProjectData
//! copy()           only the accepted part: proposed schemes (with all
//!                  their tiles) and proposed tiles are left out, and
//!                  the links that touch them; counted in Skipped
//!                  the rules of test_projects/rules.rs checked again
//! create()         the frozen copy and the first entry of the journal,
//!                  in one transaction (db/evaluations/)
//! ```
//!
//! A proposal of the AI that no person accepted never enters an
//! evaluation (P1 of `architecture.md`, user 2026-10-17).

use crate::db::evaluations::create::{self as store, Copy, Step};
use crate::db::evaluations::read as stored;
use crate::db::user::database::UserDb;
use crate::evaluation::model::{Actor, EvaluationError, EvaluationFields, JournalAction, Origin, Skipped};
use crate::standards;
use crate::standards::quote::is_on_page;
use crate::test_projects::exchange::format::Exchange;
use crate::test_projects::model::{Link, Project, ProjectData, ProposalState, TileKind};
use crate::test_projects::projects as test_projects;
use crate::test_projects::rules::{SchemeError, check_link};

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
    let data = test_projects::read::data(db, project)?.ok_or(EvaluationError::NotFound("test project"))?;

    if !data.project.ready {
        return Err(EvaluationError::Invalid(
            "project",
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
            .get(usize::try_from(number - 1).map_err(|_| EvaluationError::NotFound("document"))?)
            .copied()
            .ok_or(EvaluationError::NotFound("document"))
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
            source.found = standards::read::page(db, source.document_id, source.page)?
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
    let project = &data.project.fields;
    let mut skipped = Skipped::default();

    let schemes: Vec<_> =
        data.schemes.iter().filter(|scheme| scheme.fields.state == ProposalState::Accepted).cloned().collect();
    skipped.schemes = data.schemes.len() - schemes.len();

    let tiles: Vec<_> = data
        .tiles
        .iter()
        .filter(|tile| {
            tile.fields.state == ProposalState::Accepted && schemes.iter().any(|scheme| scheme.id == tile.scheme)
        })
        .cloned()
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
            return Err(SchemeError::Invalid("pages", reason).into());
        }
    }

    for tile in &tiles {
        tile.fields.check()?;

        if tile.fields.sources.iter().any(|source| !project.documents.contains(&source.document_id)) {
            let reason = "a source quotes a document the project does not use".to_owned();
            return Err(SchemeError::Invalid("sources", reason).into());
        }
    }

    let kinds: Vec<(i64, TileKind)> = tiles.iter().map(|tile| (tile.id, tile.fields.kind)).collect();
    let mut checked: Vec<Link> = Vec::with_capacity(links.len());

    for link in &links {
        check_link(&kinds, &checked, link.from, link.to)?;
        checked.push(*link);
    }

    let origin = Origin {
        name: project.name.trim().to_owned(),
        version: project.version.trim().to_owned(),
        author: project.author.trim().to_owned(),
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

    let copy = Copy { documents: &project.documents, schemes: &schemes, tiles: &tiles, links: &links };

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
    copy: &Copy<'_>,
    detail: &str,
) -> Result<i64, EvaluationError> {
    fields.check()?;

    if stored::name_taken(db, &fields.name)? {
        return Err(EvaluationError::NameTaken);
    }

    let step = Step { actor: &Actor::Evaluator, action: JournalAction::Imported, tile: None, field: None, detail };

    Ok(store::evaluation(db, fields, origin, copy, &step)?)
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "create_tests.rs"]
mod tests;
