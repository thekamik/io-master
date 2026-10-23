//! A new, empty catalog of OSCAL: its metadata, and the documents of the
//! standard it comes from as resources of its back matter (the file name,
//! the SHA-256), so that an exported catalog names its sources.

use crate::db::oscal as db_oscal;
use crate::db::oscal::error::OscalError;
use crate::db::oscal::ids::{new_uuid, now_date_time};
use crate::db::standards as db_standards;
use crate::db::user::database::UserDb;
use crate::model::oscal::catalog::Catalog;
use crate::model::oscal::common::{BackMatter, Hash, Metadata, Other, Resource, ResourceLink};
use crate::model::oscal::{OSCAL_VERSION, OscalDocument, OscalSummary};
use crate::model::standards::Document;

/// The most documents a catalog may come from.
pub const MAX_SOURCES: usize = 100;

/// Makes and stores a catalog without groups and controls, named `title`,
/// of the version `version` of its standard (empty: "1"), coming from the
/// documents `sources`; returns what identifies it.
pub fn create(db: &UserDb, title: &str, version: &str, sources: &[i64]) -> Result<OscalSummary, OscalError> {
    let title = title.trim();
    let version = version.trim();

    ensure!(
        !title.is_empty() && !title.contains('\n'),
        OscalError::Invalid("the catalog needs a title of one line".to_owned())
    );
    ensure!(sources.len() <= MAX_SOURCES, OscalError::Invalid(format!("at most {MAX_SOURCES} documents")));

    let mut resources = Vec::with_capacity(sources.len());

    for source in sources {
        let Some(document) = db_standards::read::document(db, *source)? else {
            return Err(OscalError::Invalid("a source does not exist".to_owned()));
        };

        resources.push(resource(&document)?);
    }

    let catalog = Catalog {
        uuid: new_uuid()?,
        metadata: metadata(title, version),
        params: Vec::new(),
        controls: Vec::new(),
        groups: Vec::new(),
        back_matter: if resources.is_empty() { None } else { Some(BackMatter { resources, other: Other::default() }) },
        other: Other::default(),
    };

    let document = OscalDocument::Catalog(catalog);
    db_oscal::create::document(db, &document)?;

    Ok(document.summary())
}

/// The metadata of a new catalog.
fn metadata(title: &str, version: &str) -> Metadata {
    Metadata {
        title: title.to_owned(),
        published: None,
        last_modified: now_date_time(),
        version: if version.is_empty() { "1".to_owned() } else { version.to_owned() },
        oscal_version: OSCAL_VERSION.to_owned(),
        props: Vec::new(),
        links: Vec::new(),
        remarks: None,
        other: Other::default(),
    }
}

/// A document of the standard as a resource: "title, part (version)", the
/// file and its SHA-256.
fn resource(document: &Document) -> Result<Resource, OscalError> {
    let fields = &document.fields;
    let mut name = fields.title.clone();

    if !fields.part.is_empty() {
        name.push_str(", ");
        name.push_str(&fields.part);
    }

    if !fields.version.is_empty() {
        name.push_str(" (");
        name.push_str(&fields.version);
        name.push(')');
    }

    Ok(Resource {
        uuid: new_uuid()?,
        title: Some(name),
        description: None,
        props: Vec::new(),
        rlinks: vec![ResourceLink {
            href: document.file_name.clone(),
            media_type: Some("application/pdf".to_owned()),
            hashes: vec![Hash { algorithm: "SHA-256".to_owned(), value: document.sha256.clone() }],
            other: Other::default(),
        }],
        other: Other::default(),
    })
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "made_tests.rs"]
mod tests;
