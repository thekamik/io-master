//! A test project as the file of its format (`format.rs`), read from
//! the database as it is stored (`projects::data`).

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde_json::{Value, json};

use crate::business::standards;
use crate::business::test_projects::projects;
use crate::db::error::DbError;
use crate::db::user::database::UserDb;

use super::format::{FORMAT, VERSION};

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
        let Some(document) = standards::documents::get(db, *id)? else {
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

        if with_pdfs && let Some(pdf) = standards::documents::pdf(db, *id)? {
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

    let links: Vec<Value> =
        data.links.iter().map(|link| json!({"from": key_of_tile(link.from), "to": key_of_tile(link.to)})).collect();

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
