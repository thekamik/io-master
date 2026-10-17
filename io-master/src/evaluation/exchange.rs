//! The file of an exported test project, so that a head of the
//! laboratory can give a project to the evaluators (steps 22 and 25 of
//! `docs/todo.md`).
//!
//! The file is JSON:
//!
//! ```text
//! {
//!   "format": "io-master-project", "version": 1,
//!   "project": {"name": ..., "version": ..., "description": ..., "author": ...},
//!   "documents": [{"key": "d1", "title": ..., "part": ..., "version": ...,
//!                  "file_name": ..., "sha256": ..., "page_count": 236,
//!                  "pdf": "<base64>"}],          "pdf" only if chosen
//!   "schemes": [{"key": "s1", "number": "9.1", "name": ..., "state": "accepted",
//!                "pages": null | {"document": "d1", "first_page": 40, "last_page": 52}}],
//!   "tiles": [{"key": "t1", "scheme": "s1", "kind": "requirement", "identifier": ..., "title": ...,
//!              "description": ..., "place": ..., "state": "accepted",
//!              "external": null | {"document": ..., "place": ...},
//!              "sections": [...], "instructions": ..., "x": 0, "y": 0,
//!              "sources": [{"document": "d1", "page": 12, "quote": ...}]}],
//!   "links": [{"from": "t1", "to": "t2"}]
//! }
//! ```
//!
//! Schemes, tiles and documents are named by keys of the file, not by the
//! numbers of a database: the numbers differ on every installation.
//! A document is found again by its SHA-256; the PDF travels in the
//! file only when the user chooses so (the laboratory decides, user,
//! 2026-10-16). Whether a quote is found is not taken from the file:
//! it is checked again against the copy of the importer.
//!
//! The JSON is built and read by hand here, as a plain value, so that
//! the types of the core carry no serialization code (CLAUDE.md).

use std::fmt;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde_json::{Map, Value, json};

use crate::db::error::DbError;
use crate::db::user::database::UserDb;

use super::documents::{self, sha256_hex};
use super::quote::is_on_page;
use super::projects;
use super::scheme::{
    External, Link, OutputSection, PageSpan, ProjectFields, ProposalState, Scheme, SchemeError, SchemeFields, Source,
    Tile, TileFields, TileKind,
};
use super::schemes::{self, StoreError};

/// The name of the format, in every file.
pub const FORMAT: &str = "io-master-project";

/// The version of the format this program writes and reads.
pub const VERSION: u64 = 1;

/// The largest file of a project that is read, in bytes.
pub const MAX_FILE: usize = 512 * 1024 * 1024;

/// A document named in a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExchangeDocument {
    pub key: String,
    pub title: String,
    pub part: String,
    pub version: String,
    pub file_name: String,
    pub sha256: String,
    pub page_count: u32,
    /// The PDF, when the file carries it.
    pub pdf: Option<Vec<u8>>,
}

/// A project read from a file, before it is stored.
#[derive(Debug, Clone, PartialEq)]
pub struct Exchange {
    /// The fields of the project, without documents.
    pub project: ProjectFields,
    pub documents: Vec<ExchangeDocument>,
    /// The schemes, numbered 1, 2, ... in the order of the file; their
    /// pages name documents by their place in `documents` + 1.
    pub schemes: Vec<Scheme>,
    /// The tiles, numbered 1, 2, ... in the order of the file; their
    /// scheme is the number of a scheme above, the sources name
    /// documents by their place in `documents` + 1.
    pub tiles: Vec<Tile>,
    pub links: Vec<Link>,
}

/// Why a file could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExchangeError {
    /// Not a file of a project, or a broken one; holds what is wrong.
    Malformed(String),
    /// A newer version of the format.
    NewerVersion(u64),
    /// The PDF of a document carried in the file is not the one it
    /// names.
    WrongPdf(String),
}

impl fmt::Display for ExchangeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExchangeError::Malformed(reason) => write!(formatter, "the file is not a valid test project: {reason}"),
            ExchangeError::NewerVersion(version) => write!(
                formatter,
                "the file is of version {version} of the format; this program reads version {VERSION}; update it"
            ),
            ExchangeError::WrongPdf(key) => write!(formatter, "the PDF carried for {key} is not the document it names"),
        }
    }
}

impl std::error::Error for ExchangeError {}

/* ---------------------------------------------------------------
 * Export
 * ------------------------------------------------------------- */

/// The file of a project; with the PDFs of its documents when
/// `with_pdfs`. `None` when there is no such project.
pub fn export(db: &UserDb, project: i64, with_pdfs: bool) -> Result<Option<Vec<u8>>, DbError> {
    let Some(data) = projects::data(db, project)? else {
        return Ok(None);
    };

    let key_of_document = |id: i64| {
        let index = data.project.fields.documents.iter().position(|document| *document == id).unwrap_or(0);
        format!("d{}", index + 1)
    };
    let key_of_scheme = |id: i64| {
        let index = data.schemes.iter().position(|scheme| scheme.id == id).unwrap_or(0);
        format!("s{}", index + 1)
    };
    let key_of_tile = |id: i64| {
        let index = data.tiles.iter().position(|tile| tile.id == id).unwrap_or(0);
        format!("t{}", index + 1)
    };

    let mut documents_json = Vec::new();

    for (index, id) in data.project.fields.documents.iter().enumerate() {
        let Some(document) = documents::get(db, *id)? else {
            continue;
        };

        let mut value = json!({
            "key": format!("d{}", index + 1),
            "title": document.fields.title,
            "part": document.fields.part,
            "version": document.fields.version,
            "file_name": document.file_name,
            "sha256": document.sha256,
            "page_count": document.page_count,
        });

        if with_pdfs && let Some(pdf) = documents::pdf(db, *id)? {
            value["pdf"] = Value::String(BASE64.encode(pdf));
        }

        documents_json.push(value);
    }

    let schemes_json: Vec<Value> = data
        .schemes
        .iter()
        .map(|scheme| {
            json!({
                "key": key_of_scheme(scheme.id),
                "number": scheme.fields.number,
                "name": scheme.fields.name,
                "state": scheme.fields.state.name(),
                "pages": scheme.fields.pages.map(|pages| json!({
                    "document": key_of_document(pages.document_id),
                    "first_page": pages.first_page,
                    "last_page": pages.last_page,
                })),
            })
        })
        .collect();

    let tiles: Vec<Value> = data
        .tiles
        .iter()
        .map(|tile| {
            let fields = &tile.fields;

            json!({
                "key": key_of_tile(tile.id),
                "scheme": key_of_scheme(tile.scheme),
                "kind": fields.kind.name(),
                "identifier": fields.identifier,
                "title": fields.title,
                "description": fields.description,
                "place": fields.place,
                "state": fields.state.name(),
                "external": fields.external.as_ref().map(|external| json!({
                    "document": external.document,
                    "place": external.place,
                })),
                "sections": fields.sections.iter().map(|section| section.name()).collect::<Vec<_>>(),
                "instructions": fields.instructions,
                "x": fields.x,
                "y": fields.y,
                "sources": fields.sources.iter().map(|source| json!({
                    "document": key_of_document(source.document_id),
                    "page": source.page,
                    "quote": source.quote,
                })).collect::<Vec<_>>(),
            })
        })
        .collect();

    let links: Vec<Value> = data
        .links
        .iter()
        .map(|link| json!({"from": key_of_tile(link.from), "to": key_of_tile(link.to)}))
        .collect();

    let file = json!({
        "format": FORMAT,
        "version": VERSION,
        "project": {
            "name": data.project.fields.name,
            "version": data.project.fields.version,
            "description": data.project.fields.description,
            "author": data.project.fields.author,
        },
        "documents": documents_json,
        "schemes": schemes_json,
        "tiles": tiles,
        "links": links,
    });

    Ok(Some(serde_json::to_vec_pretty(&file).unwrap_or_default()))
}

/* ---------------------------------------------------------------
 * Reading a file
 * ------------------------------------------------------------- */

fn malformed(reason: impl Into<String>) -> ExchangeError {
    ExchangeError::Malformed(reason.into())
}

/// A text field of an object; missing counts as empty.
fn text(object: &Map<String, Value>, name: &str) -> Result<String, ExchangeError> {
    match object.get(name) {
        None | Some(Value::Null) => Ok(String::new()),
        Some(Value::String(text)) => Ok(text.clone()),
        Some(_) => Err(malformed(format!("\"{name}\" is not a text"))),
    }
}

fn object<'a>(value: &'a Value, what: &str) -> Result<&'a Map<String, Value>, ExchangeError> {
    value.as_object().ok_or_else(|| malformed(format!("{what} is not an object")))
}

fn list<'a>(value: &'a Map<String, Value>, name: &str) -> Result<&'a [Value], ExchangeError> {
    match value.get(name) {
        None => Ok(&[]),
        Some(Value::Array(items)) => Ok(items),
        Some(_) => Err(malformed(format!("\"{name}\" is not a list"))),
    }
}

/// Reads the file of a project. The rules of a project are checked
/// when it is stored ([`store`]); here only the form of the file.
pub fn read(file: &[u8]) -> Result<Exchange, ExchangeError> {
    if file.len() > MAX_FILE {
        return Err(malformed("it is larger than 512 MiB"));
    }

    let value: Value = serde_json::from_slice(file).map_err(|error| malformed(format!("not JSON: {error}")))?;
    let root = object(&value, "the file")?;

    if root.get("format").and_then(Value::as_str) != Some(FORMAT) {
        return Err(malformed("it is not a test project of IO Master"));
    }

    let version = root.get("version").and_then(Value::as_u64).ok_or_else(|| malformed("no version"))?;

    if version > VERSION {
        return Err(ExchangeError::NewerVersion(version));
    }

    let project = object(root.get("project").unwrap_or(&Value::Null), "\"project\"")?;
    let fields = ProjectFields {
        name: text(project, "name")?,
        version: text(project, "version")?,
        description: text(project, "description")?,
        author: text(project, "author")?,
        documents: Vec::new(),
    };

    let mut documents = Vec::new();

    for item in list(root, "documents")? {
        let document = object(item, "a document")?;
        let key = text(document, "key")?;

        let pdf = match document.get("pdf") {
            None | Some(Value::Null) => None,
            Some(Value::String(encoded)) => {
                Some(BASE64.decode(encoded).map_err(|_| malformed(format!("the PDF of {key} is not base64")))?)
            }
            Some(_) => return Err(malformed(format!("the PDF of {key} is not a text"))),
        };

        let sha256 = text(document, "sha256")?.to_ascii_lowercase();

        if let Some(pdf) = &pdf
            && sha256_hex(pdf) != sha256
        {
            return Err(ExchangeError::WrongPdf(key));
        }

        documents.push(ExchangeDocument {
            title: text(document, "title")?,
            part: text(document, "part")?,
            version: text(document, "version")?,
            file_name: text(document, "file_name")?,
            sha256,
            page_count: document.get("page_count").and_then(Value::as_u64).unwrap_or(0) as u32,
            pdf,
            key,
        });
    }

    let document_number = |key: &str| {
        documents
            .iter()
            .position(|document| document.key == key)
            .map(|index| index as i64 + 1)
            .ok_or_else(|| malformed(format!("a source or a scheme names the unknown document {key}")))
    };

    let mut scheme_keys = Vec::new();
    let mut schemes = Vec::new();

    for (index, item) in list(root, "schemes")?.iter().enumerate() {
        let scheme = object(item, "a scheme")?;

        let pages = match scheme.get("pages") {
            None | Some(Value::Null) => None,
            Some(value) => {
                let pages = object(value, "\"pages\"")?;
                let page = |name: &str| pages.get(name).and_then(Value::as_u64).unwrap_or(0) as u32;

                Some(PageSpan {
                    document_id: document_number(&text(pages, "document")?)?,
                    first_page: page("first_page"),
                    last_page: page("last_page"),
                })
            }
        };

        scheme_keys.push(text(scheme, "key")?);
        schemes.push(Scheme {
            id: index as i64 + 1,
            project: 0,
            fields: SchemeFields {
                number: text(scheme, "number")?,
                name: text(scheme, "name")?,
                pages,
                state: ProposalState::from_name(&text(scheme, "state")?).unwrap_or(ProposalState::Proposed),
            },
        });
    }

    let scheme_number = |key: &str| {
        scheme_keys
            .iter()
            .position(|known| known == key)
            .map(|index| index as i64 + 1)
            .ok_or_else(|| malformed(format!("a tile names the unknown scheme {key}")))
    };

    let mut keys = Vec::new();
    let mut tiles = Vec::new();

    for (index, item) in list(root, "tiles")?.iter().enumerate() {
        let tile = object(item, "a tile")?;
        let scheme = scheme_number(&text(tile, "scheme")?)?;
        let kind = TileKind::from_name(&text(tile, "kind")?).ok_or_else(|| malformed("a tile of an unknown kind"))?;

        let mut sources = Vec::new();

        for source in list(tile, "sources")? {
            let source = object(source, "a source")?;

            sources.push(Source {
                document_id: document_number(&text(source, "document")?)?,
                page: source.get("page").and_then(Value::as_u64).unwrap_or(0) as u32,
                quote: text(source, "quote")?,
                found: false,
            });
        }

        let external = match tile.get("external") {
            None | Some(Value::Null) => None,
            Some(value) => {
                let external = object(value, "\"external\"")?;
                Some(External {
                    document: text(external, "document")?,
                    place: text(external, "place")?,
                })
            }
        };

        let sections = list(tile, "sections")?
            .iter()
            .filter_map(|section| section.as_str().and_then(OutputSection::from_name))
            .collect();

        keys.push(text(tile, "key")?);
        tiles.push(Tile {
            id: index as i64 + 1,
            scheme,
            fields: TileFields {
                kind,
                identifier: text(tile, "identifier")?,
                title: text(tile, "title")?,
                description: text(tile, "description")?,
                place: text(tile, "place")?,
                sources,
                state: ProposalState::from_name(&text(tile, "state")?).unwrap_or(ProposalState::Proposed),
                external,
                sections,
                instructions: text(tile, "instructions")?,
                x: tile.get("x").and_then(Value::as_f64).unwrap_or(0.0),
                y: tile.get("y").and_then(Value::as_f64).unwrap_or(0.0),
            },
        });
    }

    let tile_number = |key: &str| {
        keys.iter()
            .position(|known| known == key)
            .map(|index| index as i64 + 1)
            .ok_or_else(|| malformed(format!("a link names the unknown tile {key}")))
    };

    let mut links = Vec::new();

    for (index, item) in list(root, "links")?.iter().enumerate() {
        let link = object(item, "a link")?;

        links.push(Link {
            id: index as i64 + 1,
            from: tile_number(&text(link, "from")?)?,
            to: tile_number(&text(link, "to")?)?,
        });
    }

    Ok(Exchange {
        project: fields,
        documents,
        schemes,
        tiles,
        links,
    })
}

/* ---------------------------------------------------------------
 * Storing an imported project
 * ------------------------------------------------------------- */

/// The name of an imported project: its own, or with "(imported)" and
/// a number when the name is taken.
fn free_name(db: &UserDb, name: &str) -> Result<String, DbError> {
    let taken = |candidate: &str| -> Result<bool, DbError> {
        Ok(projects::list(db)?.iter().any(|project| project.fields.name.eq_ignore_ascii_case(candidate)))
    };

    if !taken(name)? {
        return Ok(name.to_owned());
    }

    for number in 1.. {
        let candidate = if number == 1 { format!("{name} (imported)") } else { format!("{name} (imported {number})") };

        if !taken(&candidate)? {
            return Ok(candidate);
        }
    }

    unreachable!("some number is free")
}

/// Stores an imported project. `documents` gives the loaded document of
/// every document of the file, in its order (found by SHA-256, or
/// loaded from the PDF the file carries). The quotes are checked
/// against these copies. Returns the number of the new project; on an
/// error nothing of it is left.
pub fn store(db: &UserDb, exchange: &Exchange, documents: &[i64]) -> Result<i64, StoreError> {
    let mut fields = exchange.project.clone();
    fields.name = free_name(db, fields.name.trim())?;
    fields.documents = documents.to_vec();

    let project = projects::create(db, &fields)?;

    match store_content(db, project, exchange, documents) {
        Ok(()) => Ok(project),
        Err(error) => {
            projects::delete(db, project)?;
            Err(error)
        }
    }
}

/// The schemes, tiles and links of an imported project.
fn store_content(db: &UserDb, project: i64, exchange: &Exchange, documents: &[i64]) -> Result<(), StoreError> {
    let document = |number: i64| {
        documents
            .get((number - 1) as usize)
            .copied()
            .ok_or(StoreError::Scheme(SchemeError::NotFound("document")))
    };

    // The number of every scheme of the file and the scheme it became.
    let mut schemes_of = Vec::with_capacity(exchange.schemes.len());

    for scheme in &exchange.schemes {
        let mut fields = scheme.fields.clone();

        if let Some(pages) = &mut fields.pages {
            pages.document_id = document(pages.document_id)?;
        }

        schemes_of.push((scheme.id, schemes::create(db, project, &fields)?));
    }

    let mut tiles = exchange.tiles.clone();

    for tile in &mut tiles {
        tile.scheme = schemes_of
            .iter()
            .find(|(number, _)| *number == tile.scheme)
            .map(|(_, id)| *id)
            .ok_or(StoreError::Scheme(SchemeError::NotFound("scheme")))?;

        for source in &mut tile.fields.sources {
            source.document_id = document(source.document_id)?;
            source.found = documents::page(db, source.document_id, source.page)?
                .is_some_and(|page| is_on_page(&source.quote, &page));
        }
    }

    schemes::replace_content(db, project, &tiles, &exchange.links)?;

    Ok(())
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "exchange_tests.rs"]
mod tests;
