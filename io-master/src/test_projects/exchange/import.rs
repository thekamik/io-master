//! Reading the file of a test project (`format.rs`) and storing a new
//! project from it. A document is found again by its SHA-256; whether a
//! quote is found is not taken from the file: it is checked again
//! against the copy of the importer.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde_json::{Map, Value};

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model::standards::sha256_hex;
use crate::model::test_projects::rules::SchemeError;
use crate::model::test_projects::{
    External, Link, OutputSection, PageSpan, ProjectFields, ProposalState, Scheme, SchemeFields, Source, Tile,
    TileFields, TileKind,
};
use crate::standards;
use crate::standards::quote::is_on_page;
use crate::test_projects::content;
use crate::test_projects::error::StoreError;
use crate::test_projects::workspace::Workspace;
use crate::test_projects::{projects, schemes};

use super::format::{Exchange, ExchangeDocument, ExchangeError, FORMAT, MAX_FILE, VERSION};

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
                Some(External { document: text(external, "document")?, place: text(external, "place")? })
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

    Ok(Exchange { project: fields, documents, schemes, tiles, links })
}

/* ---------------------------------------------------------------
 * Storing an imported project
 * ------------------------------------------------------------- */

/// The name of an imported project: its own, or with "(imported)" and
/// a number when the name is taken.
fn free_name(db: &UserDb, name: &str) -> Result<String, DbError> {
    let taken = |candidate: &str| -> Result<bool, DbError> {
        Ok(projects::read::list(db)?.iter().any(|project| project.fields.name.eq_ignore_ascii_case(candidate)))
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

    let project = projects::create::project(db, &fields)?;

    // The new project is opened in a workspace of its own, so that its
    // schemes are added by the same rules as by hand.
    let mut workspace = Workspace::default();

    match store_content(db, &mut workspace, project, exchange, documents) {
        Ok(()) => Ok(project),
        Err(error) => {
            projects::delete::project(db, &mut workspace, project)?;
            Err(error)
        }
    }
}

/// The schemes, tiles and links of an imported project.
fn store_content(
    db: &UserDb,
    workspace: &mut Workspace,
    project: i64,
    exchange: &Exchange,
    documents: &[i64],
) -> Result<(), StoreError> {
    let open = workspace.open(db, project)?;
    let document = |number: i64| {
        documents.get((number - 1) as usize).copied().ok_or(StoreError::Scheme(SchemeError::NotFound("document")))
    };

    // The number of every scheme of the file and the scheme it became.
    let mut schemes_of = Vec::with_capacity(exchange.schemes.len());

    for scheme in &exchange.schemes {
        let mut fields = scheme.fields.clone();

        if let Some(pages) = &mut fields.pages {
            pages.document_id = document(pages.document_id)?;
        }

        schemes_of.push((scheme.id, schemes::create::scheme(db, open, &fields)?));
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
            source.found = standards::read::page(db, source.document_id, source.page)?
                .is_some_and(|page| is_on_page(&source.quote, &page));
        }
    }

    content::replace(db, project, &tiles, &exchange.links)?;

    Ok(())
}

// Unit tests, in their own file to keep this one readable. They
// cover the whole folder (format, export, import).
#[cfg(test)]
#[path = "import_tests.rs"]
mod tests;
