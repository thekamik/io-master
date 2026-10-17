//! The rows of the test projects and the Rust types they become
//! (`evaluation/scheme.rs`), in both directions: reading a row into a
//! type, and the values a type writes into a row.

use rusqlite::{Connection, Row, params};

use crate::evaluation::scheme::{
    External, Link, OutputSection, PageSpan, ProposalState, Scheme, SchemeFields, Source, Tile, TileFields, TileKind,
};

/// The columns of a project, in the order of [`project_from_row`].
pub(super) const PROJECT_COLUMNS: &str = "id, name, version, description, author, ready";

/// A project without its documents: id, name, version, description,
/// author, ready (`read.rs` adds the documents).
pub(super) type ProjectRow = (i64, String, String, String, String, bool);

pub(super) fn project_from_row(row: &Row<'_>) -> rusqlite::Result<ProjectRow> {
    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?))
}

/// Writes the documents of a project, replacing the earlier list.
pub(super) fn write_documents(connection: &Connection, id: i64, documents: &[i64]) -> rusqlite::Result<()> {
    connection.execute("DELETE FROM project_documents WHERE project_id = ?1", [id])?;

    for (position, document) in documents.iter().enumerate() {
        connection.execute(
            "INSERT OR IGNORE INTO project_documents (project_id, document_id, position) VALUES (?1, ?2, ?3)",
            params![id, document, position as i64],
        )?;
    }

    Ok(())
}

/// The columns of a scheme, in the order of [`scheme_from_row`].
pub(super) const SCHEME_COLUMNS: &str = "id, project_id, number, name, state, document_id, first_page, last_page";

pub(super) fn scheme_from_row(row: &Row<'_>) -> rusqlite::Result<Scheme> {
    let state: String = row.get(4)?;
    let document: Option<i64> = row.get(5)?;
    let first: Option<u32> = row.get(6)?;
    let last: Option<u32> = row.get(7)?;

    Ok(Scheme {
        id: row.get(0)?,
        project: row.get(1)?,
        fields: SchemeFields {
            number: row.get(2)?,
            name: row.get(3)?,
            pages: match (document, first, last) {
                (Some(document_id), Some(first_page), Some(last_page)) => Some(PageSpan {
                    document_id,
                    first_page,
                    last_page,
                }),
                _ => None,
            },
            state: ProposalState::from_name(&state).unwrap_or(ProposalState::Proposed),
        },
    })
}

/// The values of the columns of a scheme (?2 to ?7).
pub(super) fn write_scheme(
    connection: &Connection,
    sql: &str,
    first: i64,
    fields: &SchemeFields,
) -> rusqlite::Result<usize> {
    connection.execute(
        sql,
        params![
            first,
            fields.number.trim(),
            fields.name.trim(),
            fields.state.name(),
            fields.pages.map(|pages| pages.document_id),
            fields.pages.map(|pages| pages.first_page),
            fields.pages.map(|pages| pages.last_page),
        ],
    )
}

/// The columns of a tile `t`, in the order of [`tile_from_row`].
pub(super) const TILE_COLUMNS: &str = "t.id, t.scheme_id, t.kind, t.identifier, t.title, t.description, t.place, \
t.state, t.external_document, t.external_place, t.sections, t.instructions, t.x, t.y";

pub(super) fn tile_from_row(row: &Row<'_>) -> rusqlite::Result<Tile> {
    let kind: String = row.get(2)?;
    let state: String = row.get(7)?;
    let external_document: Option<String> = row.get(8)?;
    let external_place: Option<String> = row.get(9)?;
    let sections: String = row.get(10)?;

    Ok(Tile {
        id: row.get(0)?,
        scheme: row.get(1)?,
        fields: TileFields {
            // Only valid names are written; a database changed by hand
            // falls back to the safe side.
            kind: TileKind::from_name(&kind).unwrap_or(TileKind::Requirement),
            identifier: row.get(3)?,
            title: row.get(4)?,
            description: row.get(5)?,
            place: row.get(6)?,
            sources: Vec::new(),
            state: ProposalState::from_name(&state).unwrap_or(ProposalState::Proposed),
            external: external_document.map(|document| External {
                document,
                place: external_place.unwrap_or_default(),
            }),
            sections: sections.split(',').filter_map(OutputSection::from_name).collect(),
            instructions: row.get(11)?,
            x: row.get(12)?,
            y: row.get(13)?,
        },
    })
}

/// The statement that adds a tile to the scheme `?1`.
pub(super) const INSERT_TILE: &str = "INSERT INTO tiles (scheme_id, kind, identifier, title, description, place, \
state, external_document, external_place, sections, instructions, x, y) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, \
?10, ?11, ?12, ?13)";

/// The values of the columns of a tile (?2 to ?13).
pub(super) fn write_tile(
    connection: &Connection,
    sql: &str,
    first: i64,
    fields: &TileFields,
) -> rusqlite::Result<usize> {
    let sections: Vec<&str> = fields.sections.iter().map(|section| section.name()).collect();

    connection.execute(
        sql,
        params![
            first,
            fields.kind.name(),
            fields.identifier.trim(),
            fields.title.trim(),
            fields.description,
            fields.place.trim(),
            fields.state.name(),
            fields.external.as_ref().map(|external| external.document.trim()),
            fields.external.as_ref().map(|external| external.place.trim()),
            sections.join(","),
            fields.instructions,
            fields.x,
            fields.y,
        ],
    )
}

/// Writes the sources of a tile, replacing the earlier ones.
pub(super) fn write_sources(connection: &Connection, tile: i64, sources: &[Source]) -> rusqlite::Result<()> {
    connection.execute("DELETE FROM tile_sources WHERE tile_id = ?1", [tile])?;

    for source in sources {
        connection.execute(
            "INSERT INTO tile_sources (tile_id, document_id, page, quote, found) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![tile, source.document_id, source.page, source.quote, source.found],
        )?;
    }

    Ok(())
}

/// A link from a row of `id, from_tile, to_tile`.
pub(super) fn link_from_row(row: &Row<'_>) -> rusqlite::Result<Link> {
    Ok(Link {
        id: row.get(0)?,
        from: row.get(1)?,
        to: row.get(2)?,
    })
}
