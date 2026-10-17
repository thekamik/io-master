//! The test projects open for editing: the objects in memory (user,
//! 2026-10-17).
//!
//! ```text
//! db/test_projects/  ──open()──►  OpenProject in memory
//!                                  data       the project as it is now, equal to the database
//!                                  versions   earlier contents (tiles, links), newest last, for "Undo"
//!                    ◄──every change, at once (write-through)── projects/ schemes/ tiles/ links/
//! ```
//!
//! **Where the information is.** While a project is open, its current
//! state is in [`OpenProject::data`], and it is the same as in the
//! database: every change is written through at once, in one
//! transaction, by `db/test_projects/` and only then made in memory; a
//! change the database refuses is not made at all. The earlier
//! versions are only in memory: they are lost when the evaluator logs
//! out, as the "Undo" of an editor is.
//!
//! **One version per action of the user.** The CRUD functions do not
//! keep versions themselves; an action of the user (add a tile, an
//! analysis of a whole chapter) is wrapped in [`OpenProject::change`],
//! which keeps the content before it once. A version holds the tiles
//! and the links: the schemes and the fields of the project are not
//! undone, and deleting a scheme forgets the versions (its tiles could
//! not come back without it).

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use crate::db::test_projects as db_test_projects;
use crate::db::user::database::UserDb;
use crate::model::text::Entity;

use super::projects;
use crate::business::test_projects::error::StoreError;
use crate::model::test_projects::rules::ProjectError;
use crate::model::test_projects::{Link, ProjectData, Tile};

/// How many changes of one test project can be undone.
pub const VERSIONS: usize = 50;

/// The content of a project that "Undo" brings back.
#[derive(Debug, Clone, PartialEq)]
struct Version {
    tiles: Vec<Tile>,
    links: Vec<Link>,
}

/// A test project open for editing.
#[derive(Debug)]
pub struct OpenProject {
    data: ProjectData,
    versions: Vec<Version>,
}

impl OpenProject {
    /// The project as it is now (the same as in the database).
    pub fn data(&self) -> &ProjectData {
        &self.data
    }

    /// The project, to be changed by the CRUD functions of
    /// `business/test_projects/` after the database has stored the change.
    pub(in crate::business::test_projects) fn data_mut(&mut self) -> &mut ProjectData {
        &mut self.data
    }

    /// Whether a change can be undone.
    pub fn can_undo(&self) -> bool {
        !self.versions.is_empty()
    }

    /// Runs one action of the user, `work`, keeping the content before
    /// it as one version for "Undo". When `work` fails, the version is
    /// dropped: nothing was changed (the CRUD functions change memory
    /// only after the database has stored the change).
    pub fn change<T, E>(&mut self, work: impl FnOnce(&mut OpenProject) -> Result<T, E>) -> Result<T, E> {
        let before = Version { tiles: self.data.tiles.clone(), links: self.data.links.clone() };

        let result = work(self)?;

        // Only a change that changed something is a version.
        if before.tiles != self.data.tiles || before.links != self.data.links {
            self.versions.push(before);

            if self.versions.len() > VERSIONS {
                self.versions.remove(0);
            }
        }

        Ok(result)
    }

    /// Keeps the content now as one version, before a long work made of
    /// many steps (the analysis of chapters by the AI, which waits for
    /// the model between them): one "Undo" takes the whole work back.
    pub fn remember(&mut self) {
        self.versions.push(Version { tiles: self.data.tiles.clone(), links: self.data.links.clone() });

        if self.versions.len() > VERSIONS {
            self.versions.remove(0);
        }
    }

    /// Undoes the last change: its version is written to the database
    /// (with the numbers the tiles and links had) and becomes the
    /// project in memory. Refused when there is nothing to undo.
    pub fn undo(&mut self, db: &UserDb) -> Result<(), StoreError> {
        let Some(version) = self.versions.last() else {
            return Err(ProjectError::NothingToUndo.into());
        };

        let project = self.data.project.id;

        if db_test_projects::update::restore_content(db, project, &version.tiles, &version.links).is_ok() {
            let version = self.versions.pop().expect("a version was there");
            self.data.tiles = version.tiles;
            self.data.links = version.links;

            return Ok(());
        }

        // A number of the version was given to a tile or link of
        // another project meanwhile (SQLite may give a freed number
        // again), so the database refused it, and nothing was changed.
        // The version is written with new numbers, and the project is
        // read back, so that memory and the database stay the same.
        // The earlier versions name the old numbers, so they are
        // forgotten too.
        db_test_projects::update::replace_content(db, project, &version.tiles, &version.links)?;
        self.versions.clear();
        self.data = projects::data(db, project)?.ok_or(StoreError::Rule(ProjectError::NotFound(Entity::Project)))?;

        Ok(())
    }

    /// Forgets the versions (a scheme was deleted).
    pub(in crate::business::test_projects) fn forget_versions(&mut self) {
        self.versions.clear();
    }
}

/// The projects open for editing of the logged-in evaluator.
#[derive(Debug, Default)]
pub struct ProjectWorkspace {
    open: HashMap<i64, OpenProject>,
}

impl ProjectWorkspace {
    /// The open project `id`; read from the database when it is not
    /// open yet.
    pub fn open(&mut self, db: &UserDb, id: i64) -> Result<&mut OpenProject, StoreError> {
        match self.open.entry(id) {
            Entry::Occupied(open) => Ok(open.into_mut()),
            Entry::Vacant(place) => {
                let data = projects::data(db, id)?.ok_or(StoreError::Rule(ProjectError::NotFound(Entity::Project)))?;

                Ok(place.insert(OpenProject { data, versions: Vec::new() }))
            }
        }
    }

    /// Whether project `id` is open, and its versions.
    pub fn get(&self, id: i64) -> Option<&OpenProject> {
        self.open.get(&id)
    }

    /// Closes project `id` (deleted, or replaced from outside); its
    /// versions are forgotten.
    pub fn close(&mut self, id: i64) {
        self.open.remove(&id);
    }

    /// Takes project `id` out of the workspace (for the tests, which
    /// work on one project at a time).
    #[cfg(test)]
    pub fn take(&mut self, id: i64) -> Option<OpenProject> {
        self.open.remove(&id)
    }

    /// Closes every project: the evaluator logged out.
    pub fn clear(&mut self) {
        self.open.clear();
    }
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "workspace_tests.rs"]
mod tests;
