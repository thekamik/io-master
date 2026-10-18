//! The catalogs of the evaluator as a resource (step 41.3): a standard
//! (its criteria, a methodology with its work units, a test
//! specification) made in the program, from the documents of the
//! standard (PDF), group by group and control by control. A catalog of
//! NIST or of a laboratory is imported instead (`documents.rs`).
//!
//! ```text
//! create()           POST    /catalogs                         a new, empty catalog
//! get()              GET     /catalogs/{uuid}
//! add_group()        POST    /catalogs/{uuid}/groups           at the top or in a group
//! update_group()     PUT     /catalogs/{uuid}/groups/{id}      its own fields; its content stays
//! add_control()      POST    /catalogs/{uuid}/controls         in a group or a control
//! update_control()   PUT     /catalogs/{uuid}/controls/{id}    its own fields; its enhancements stay
//! delete_object()    DELETE  /catalogs/{uuid}/objects/{id}     a group or a control, with its content
//! ```
//!
//! Every change is checked against the schema of NIST and written to
//! the database at once (`db/oscal/`), with a new `last-modified`. The
//! documents of the standard a catalog comes from are resources of its
//! back matter, found by the SHA-256 of their files; the passages an
//! object comes from are evaluation support (`quotes.rs`).

use std::collections::HashSet;

use crate::business::standards;
use crate::db::evaluation_support as db_evaluation_support;
use crate::db::oscal as db_oscal;
use crate::db::user::database::UserDb;
use crate::model::oscal::catalog::{Catalog, Control, Group};
use crate::model::oscal::common::{BackMatter, Hash, Metadata, Resource, ResourceLink};
use crate::model::oscal::{OSCAL_VERSION, OscalDocument, OscalSummary};

use super::documents::OscalError;
use super::ids::{new_uuid, now_date_time};

/// Where a new object goes: the top of the catalog, a group, or a
/// control (for an enhancement).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Place {
    Top,
    Group(String),
    Control(String),
}

/// Makes a new catalog without groups and controls, named `title`, of
/// the version `version` of its standard, coming from the documents
/// `sources` (loaded PDFs of the standard); returns what identifies it.
pub fn create(db: &UserDb, title: &str, version: &str, sources: &[i64]) -> Result<OscalSummary, OscalError> {
    let title = title.trim();
    let version = version.trim();

    if title.is_empty() || title.contains('\n') {
        return Err(OscalError::Invalid("the catalog needs a title of one line".to_owned()));
    }

    let mut resources = Vec::new();

    for source in sources {
        let document =
            standards::documents::get(db, *source)?.ok_or(OscalError::Invalid("a source does not exist".to_owned()))?;
        let fields = &document.fields;
        let mut name = fields.title.clone();

        if !fields.part.is_empty() {
            name.push_str(&format!(", {}", fields.part));
        }

        if !fields.version.is_empty() {
            name.push_str(&format!(" ({})", fields.version));
        }

        resources.push(Resource {
            uuid: new_uuid()?,
            title: Some(name),
            description: None,
            props: Vec::new(),
            rlinks: vec![ResourceLink {
                href: document.file_name.clone(),
                media_type: Some("application/pdf".to_owned()),
                hashes: vec![Hash { algorithm: "SHA-256".to_owned(), value: document.sha256.clone() }],
                other: Default::default(),
            }],
            other: Default::default(),
        });
    }

    let catalog = Catalog {
        uuid: new_uuid()?,
        metadata: Metadata {
            title: title.to_owned(),
            published: None,
            last_modified: now_date_time(),
            version: if version.is_empty() { "1".to_owned() } else { version.to_owned() },
            oscal_version: OSCAL_VERSION.to_owned(),
            props: Vec::new(),
            links: Vec::new(),
            remarks: None,
            other: Default::default(),
        },
        params: Vec::new(),
        controls: Vec::new(),
        groups: Vec::new(),
        back_matter: if resources.is_empty() {
            None
        } else {
            Some(BackMatter { resources, other: Default::default() })
        },
        other: Default::default(),
    };

    let document = OscalDocument::Catalog(catalog);
    db_oscal::create::document(db, &document)?;

    Ok(document.summary())
}

/// Catalog `uuid`.
pub fn get(db: &UserDb, uuid: &str) -> Result<Catalog, OscalError> {
    match db_oscal::read::document(db, uuid)? {
        Some(OscalDocument::Catalog(catalog)) => Ok(catalog),
        Some(_) => Err(OscalError::WrongModel),
        None => Err(OscalError::NotFound),
    }
}

/// Writes a changed catalog: a new `last-modified`, checked against the
/// schema of NIST, stored.
fn save(db: &UserDb, mut catalog: Catalog) -> Result<(), OscalError> {
    catalog.metadata.last_modified = now_date_time();
    db_oscal::update::document(db, &OscalDocument::Catalog(catalog))?;

    Ok(())
}

/// The ids of every group and control of a catalog.
fn ids(catalog: &Catalog) -> HashSet<String> {
    fn group(found: &mut HashSet<String>, group_: &Group) {
        if let Some(id) = &group_.id {
            found.insert(id.clone());
        }

        group_.controls.iter().for_each(|inner| control(found, inner));
        group_.groups.iter().for_each(|inner| group(found, inner));
    }

    fn control(found: &mut HashSet<String>, control_: &Control) {
        found.insert(control_.id.clone());
        control_.controls.iter().for_each(|inner| control(found, inner));
    }

    let mut found = HashSet::new();
    catalog.controls.iter().for_each(|inner| control(&mut found, inner));
    catalog.groups.iter().for_each(|inner| group(&mut found, inner));

    found
}

/// The group `id` of the catalog, to be changed.
fn group_mut<'a>(groups: &'a mut [Group], id: &str) -> Option<&'a mut Group> {
    for group in groups {
        if group.id.as_deref() == Some(id) {
            return Some(group);
        }

        if let Some(found) = group_mut(&mut group.groups, id) {
            return Some(found);
        }
    }

    None
}

/// The control `id` among `controls` and their enhancements.
fn control_in<'a>(controls: &'a mut [Control], id: &str) -> Option<&'a mut Control> {
    for control in controls {
        if control.id == id {
            return Some(control);
        }

        if let Some(found) = control_in(&mut control.controls, id) {
            return Some(found);
        }
    }

    None
}

/// The control `id` of the catalog, to be changed.
fn control_mut<'a>(catalog: &'a mut Catalog, id: &str) -> Option<&'a mut Control> {
    fn in_groups<'a>(groups: &'a mut [Group], id: &str) -> Option<&'a mut Control> {
        for group in groups {
            if let Some(found) = control_in(&mut group.controls, id) {
                return Some(found);
            }

            if let Some(found) = in_groups(&mut group.groups, id) {
                return Some(found);
            }
        }

        None
    }

    control_in(&mut catalog.controls, id).or_else(|| in_groups(&mut catalog.groups, id))
}

/// The id of a new object, checked: a token of OSCAL, not used yet in
/// the catalog.
fn new_id(catalog: &Catalog, id: &str) -> Result<(), OscalError> {
    if id.is_empty() {
        return Err(OscalError::Invalid("the object needs an id".to_owned()));
    }

    if ids(catalog).contains(id) {
        return Err(OscalError::IdTaken(id.to_owned()));
    }

    Ok(())
}

/// Adds `group` (without its own content: that is added later) at
/// `place` (the top or a group) of catalog `uuid`.
pub fn add_group(db: &UserDb, uuid: &str, place: &Place, mut group: Group) -> Result<(), OscalError> {
    let mut catalog = get(db, uuid)?;
    let id = group.id.clone().ok_or(OscalError::Invalid("the group needs an id".to_owned()))?;
    new_id(&catalog, &id)?;

    group.groups.clear();
    group.controls.clear();

    match place {
        Place::Top => catalog.groups.push(group),
        Place::Group(parent) => {
            group_mut(&mut catalog.groups, parent).ok_or(OscalError::NoSuchObject(parent.clone()))?.groups.push(group)
        }
        Place::Control(_) => return Err(OscalError::Invalid("a group goes into the catalog or a group".to_owned())),
    }

    save(db, catalog)
}

/// Changes the own fields of group `id` (id, title, class, props,
/// parts, params); its groups and controls stay.
pub fn update_group(db: &UserDb, uuid: &str, id: &str, mut group: Group) -> Result<(), OscalError> {
    let mut catalog = get(db, uuid)?;
    let new = group.id.clone().ok_or(OscalError::Invalid("the group needs an id".to_owned()))?;

    if new != id {
        new_id(&catalog, &new)?;
    }

    let stored = group_mut(&mut catalog.groups, id).ok_or(OscalError::NoSuchObject(id.to_owned()))?;
    group.groups = std::mem::take(&mut stored.groups);
    group.controls = std::mem::take(&mut stored.controls);
    *stored = group;

    save(db, catalog)
}

/// Adds `control` (without its enhancements) at `place` (the top, a
/// group, or a control for an enhancement) of catalog `uuid`.
pub fn add_control(db: &UserDb, uuid: &str, place: &Place, mut control: Control) -> Result<(), OscalError> {
    let mut catalog = get(db, uuid)?;
    new_id(&catalog, &control.id)?;
    control.controls.clear();

    match place {
        Place::Top => catalog.controls.push(control),
        Place::Group(parent) => group_mut(&mut catalog.groups, parent)
            .ok_or(OscalError::NoSuchObject(parent.clone()))?
            .controls
            .push(control),
        Place::Control(parent) => {
            control_mut(&mut catalog, parent).ok_or(OscalError::NoSuchObject(parent.clone()))?.controls.push(control)
        }
    }

    save(db, catalog)
}

/// Changes the own fields of control `id`; its enhancements stay.
pub fn update_control(db: &UserDb, uuid: &str, id: &str, mut control: Control) -> Result<(), OscalError> {
    let mut catalog = get(db, uuid)?;

    if control.id != id {
        new_id(&catalog, &control.id)?;
    }

    let stored = control_mut(&mut catalog, id).ok_or(OscalError::NoSuchObject(id.to_owned()))?;
    control.controls = std::mem::take(&mut stored.controls);
    *stored = control;

    save(db, catalog)
}

/// Deletes the group or control `id` of catalog `uuid` with its
/// content, and the quotes of everything deleted.
pub fn delete_object(db: &UserDb, uuid: &str, id: &str) -> Result<(), OscalError> {
    /// Removes the control `id` from `controls` or their enhancements;
    /// returns whether it was there.
    fn remove_control(controls: &mut Vec<Control>, id: &str) -> bool {
        let before = controls.len();
        controls.retain(|control| control.id != id);

        before != controls.len() || controls.iter_mut().any(|control| remove_control(&mut control.controls, id))
    }

    /// Removes the group or control `id` from `groups` and their
    /// content; returns whether it was there.
    fn remove(groups: &mut Vec<Group>, id: &str) -> bool {
        let before = groups.len();
        groups.retain(|group| group.id.as_deref() != Some(id));

        before != groups.len()
            || groups.iter_mut().any(|group| remove_control(&mut group.controls, id) || remove(&mut group.groups, id))
    }

    let mut catalog = get(db, uuid)?;
    let before = ids(&catalog);

    let removed = remove_control(&mut catalog.controls, id) || remove(&mut catalog.groups, id);

    if !removed {
        return Err(OscalError::NoSuchObject(id.to_owned()));
    }

    let after = ids(&catalog);
    let gone: Vec<String> = before.difference(&after).cloned().collect();

    save(db, catalog)?;
    db_evaluation_support::delete::quotes_of(db, uuid, &gone)?;

    Ok(())
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "catalogs_tests.rs"]
mod tests;
