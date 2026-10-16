//! The test projects of an evaluator, kept in their encrypted database
//! (`db/user_db.rs`):
//!
//! ```text
//! projects           name, version, description, author, ready
//! project_documents  the documents of the standard a project uses
//! ```
//!
//! The schemes of a project, their tiles and the links are stored by
//! `schemes.rs`; [`data`] reads a whole project at once. Every change
//! is checked by the rules of `scheme.rs` before it is stored; changes
//! of several rows are made in one transaction, so a project is never
//! left half changed.

use rusqlite::{OptionalExtension, Row, params};

use crate::db::db::DbError;
use crate::db::user_db::UserDb;

use super::documents;
use super::scheme::{Link, Project, ProjectFields, Scheme, SchemeError, Tile};
use super::schemes::{self, StoreError};

/// A project with everything in it.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectData {
    pub project: Project,
    /// In the order of the list of the project.
    pub schemes: Vec<Scheme>,
    /// The tiles of all its schemes.
    pub tiles: Vec<Tile>,
    pub links: Vec<Link>,
}

fn project_from_row(row: &Row<'_>) -> rusqlite::Result<(i64, String, String, String, String, bool)> {
    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?))
}

/// The documents a project uses.
fn project_documents(db: &UserDb, id: i64) -> Result<Vec<i64>, DbError> {
    let mut statement = db
        .connection()
        .prepare("SELECT document_id FROM project_documents WHERE project_id = ?1 ORDER BY position")?;
    let ids = statement.query_map([id], |row| row.get(0))?.collect::<Result<Vec<_>, _>>()?;

    Ok(ids)
}

fn project(db: &UserDb, row: (i64, String, String, String, String, bool)) -> Result<Project, DbError> {
    let (id, name, version, description, author, ready) = row;

    Ok(Project {
        id,
        fields: ProjectFields {
            name,
            version,
            description,
            author,
            documents: project_documents(db, id)?,
        },
        ready,
    })
}

/// Checks the fields and the documents of a project, and the name
/// against the other projects.
fn check_project(db: &UserDb, id: Option<i64>, fields: &ProjectFields) -> Result<(), StoreError> {
    fields.check()?;

    for document in &fields.documents {
        if documents::get(db, *document)?.is_none() {
            return Err(SchemeError::NotFound("document").into());
        }
    }

    let taken: Option<i64> = db
        .connection()
        .query_row(
            "SELECT id FROM projects WHERE name = ?1 COLLATE NOCASE",
            [fields.name.trim()],
            |row| row.get(0),
        )
        .optional()?;

    if taken.is_some_and(|other| Some(other) != id) {
        return Err(SchemeError::NameTaken.into());
    }

    Ok(())
}

/// Writes the documents of a project, replacing the earlier list.
fn write_documents(connection: &rusqlite::Connection, id: i64, documents: &[i64]) -> rusqlite::Result<()> {
    connection.execute("DELETE FROM project_documents WHERE project_id = ?1", [id])?;

    for (position, document) in documents.iter().enumerate() {
        connection.execute(
            "INSERT OR IGNORE INTO project_documents (project_id, document_id, position) VALUES (?1, ?2, ?3)",
            params![id, document, position as i64],
        )?;
    }

    Ok(())
}

/// Makes a project; returns its number.
pub fn create(db: &UserDb, fields: &ProjectFields) -> Result<i64, StoreError> {
    check_project(db, None, fields)?;

    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute(
        "INSERT INTO projects (name, version, description, author, ready) VALUES (?1, ?2, ?3, ?4, 0)",
        params![fields.name.trim(), fields.version.trim(), fields.description, fields.author.trim()],
    )?;

    let id = transaction.last_insert_rowid();
    write_documents(&transaction, id, &fields.documents)?;
    transaction.commit()?;

    Ok(id)
}

/// Every project, by name.
pub fn list(db: &UserDb) -> Result<Vec<Project>, DbError> {
    let mut statement = db
        .connection()
        .prepare("SELECT id, name, version, description, author, ready FROM projects ORDER BY name COLLATE NOCASE, id")?;
    let rows = statement.query_map([], project_from_row)?.collect::<Result<Vec<_>, _>>()?;

    rows.into_iter().map(|row| project(db, row)).collect()
}

/// One project, without its schemes; `None` when there is no such.
pub fn get(db: &UserDb, id: i64) -> Result<Option<Project>, DbError> {
    let row = db
        .connection()
        .query_row(
            "SELECT id, name, version, description, author, ready FROM projects WHERE id = ?1",
            [id],
            project_from_row,
        )
        .optional()?;

    row.map(|row| project(db, row)).transpose()
}

/// The project, or the error that it does not exist.
pub fn existing(db: &UserDb, id: i64) -> Result<Project, StoreError> {
    get(db, id)?.ok_or(StoreError::Scheme(SchemeError::NotFound("project")))
}

/// Changes the fields of a project. A document still quoted by a tile,
/// or holding the pages of a scheme, cannot be taken away.
pub fn update(db: &UserDb, id: i64, fields: &ProjectFields) -> Result<(), StoreError> {
    existing(db, id)?;
    check_project(db, Some(id), fields)?;

    let used: Vec<i64> = {
        let mut statement = db.connection().prepare(
            "SELECT s.document_id FROM tile_sources s JOIN tiles t ON t.id = s.tile_id \
             JOIN schemes c ON c.id = t.scheme_id WHERE c.project_id = ?1 \
             UNION SELECT document_id FROM schemes WHERE project_id = ?1 AND document_id IS NOT NULL",
        )?;
        statement.query_map([id], |row| row.get(0))?.collect::<Result<Vec<_>, _>>()?
    };

    if used.iter().any(|document| !fields.documents.contains(document)) {
        return Err(SchemeError::Invalid(
            "documents",
            "a tile quotes, or a scheme covers pages of, a document taken away from the project; change those first"
                .to_owned(),
        )
        .into());
    }

    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute(
        "UPDATE projects SET name = ?1, version = ?2, description = ?3, author = ?4 WHERE id = ?5",
        params![fields.name.trim(), fields.version.trim(), fields.description, fields.author.trim(), id],
    )?;
    write_documents(&transaction, id, &fields.documents)?;
    transaction.commit()?;

    Ok(())
}

/// Marks a project ready, or a draft again.
pub fn set_ready(db: &UserDb, id: i64, ready: bool) -> Result<(), StoreError> {
    existing(db, id)?;
    db.connection().execute("UPDATE projects SET ready = ?1 WHERE id = ?2", params![ready, id])?;

    Ok(())
}

/// Deletes a project with its schemes, tiles and links; returns
/// whether it existed.
pub fn delete(db: &UserDb, id: i64) -> Result<bool, DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute(
        "DELETE FROM tile_sources WHERE tile_id IN \
         (SELECT t.id FROM tiles t JOIN schemes c ON c.id = t.scheme_id WHERE c.project_id = ?1)",
        [id],
    )?;
    transaction.execute("DELETE FROM links WHERE project_id = ?1", [id])?;
    transaction.execute(
        "DELETE FROM tiles WHERE scheme_id IN (SELECT id FROM schemes WHERE project_id = ?1)",
        [id],
    )?;
    transaction.execute("DELETE FROM schemes WHERE project_id = ?1", [id])?;
    transaction.execute("DELETE FROM project_documents WHERE project_id = ?1", [id])?;
    let deleted = transaction.execute("DELETE FROM projects WHERE id = ?1", [id])? > 0;
    transaction.commit()?;

    Ok(deleted)
}

/// A project with its schemes, tiles and links.
pub fn data(db: &UserDb, id: i64) -> Result<Option<ProjectData>, DbError> {
    let Some(project) = get(db, id)? else {
        return Ok(None);
    };

    Ok(Some(ProjectData {
        schemes: schemes::of_project(db, id)?,
        tiles: schemes::project_tiles(db, id)?,
        links: schemes::links(db, id)?,
        project,
    }))
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "projects_tests.rs"]
mod tests;
