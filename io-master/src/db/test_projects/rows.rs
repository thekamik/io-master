//! The rows of the test projects and the Rust types they become
//! (`model/test_projects.rs`).
//!
//! **Columns are read by their names**, so the order of a `SELECT`
//! does not matter. **A value the program never writes** (a kind, a
//! state, a section) is an error (`DbError::Corrupt`), never replaced
//! in silence by one that looks right.
//!
//! The pieces marked `pub(in crate::db)` are also used by
//! `db/evaluations/`, whose frozen copy has the same tiles and sources
//! in tables of its own: one reading of a tile for both.

use std::collections::HashMap;

use rusqlite::{Connection, Row};

use crate::db::error::named;
use crate::model::test_projects::{
    External, Link, OutputSection, PageSpan, Project, ProjectFields, ProposalState, Scheme, SchemeFields, Source, Tile,
    TileFields, TileKind,
};

/// A project from a row of `projects`, without its documents
/// (`read.rs` adds them).
pub(super) fn project_from_row(row: &Row<'_>) -> rusqlite::Result<Project> {
    Ok(Project {
        id: row.get("id")?,
        fields: ProjectFields {
            name: row.get("name")?,
            version: row.get("version")?,
            description: row.get("description")?,
            author: row.get("author")?,
            documents: Vec::new(),
        },
        ready: row.get("ready")?,
    })
}

/// The pages of a scheme: all three columns, or none of them.
pub(in crate::db) fn pages_from_row(row: &Row<'_>) -> rusqlite::Result<Option<PageSpan>> {
    let document: Option<i64> = row.get("document_id")?;
    let first: Option<u32> = row.get("first_page")?;
    let last: Option<u32> = row.get("last_page")?;

    Ok(match (document, first, last) {
        (Some(document_id), Some(first_page), Some(last_page)) => Some(PageSpan { document_id, first_page, last_page }),
        _ => None,
    })
}

/// A scheme from a row of `schemes`.
pub(super) fn scheme_from_row(row: &Row<'_>) -> rusqlite::Result<Scheme> {
    let state: String = row.get("state")?;

    Ok(Scheme {
        id: row.get("id")?,
        project: row.get("project_id")?,
        fields: SchemeFields {
            number: row.get("number")?,
            name: row.get("name")?,
            pages: pages_from_row(row)?,
            state: named("schemes", "state", &state, ProposalState::from_name)?,
        },
    })
}

/// A tile from a row of the table `table` (`tiles`, or
/// `evaluation_tiles` of the frozen copy), without its sources, in the
/// state `state` (read by the caller: only `tiles` has the column).
pub(in crate::db) fn tile_from_row(row: &Row<'_>, table: &'static str, state: ProposalState) -> rusqlite::Result<Tile> {
    let kind: String = row.get("kind")?;
    let external_document: Option<String> = row.get("external_document")?;
    let external_place: Option<String> = row.get("external_place")?;
    let sections: String = row.get("sections")?;

    Ok(Tile {
        id: row.get("id")?,
        scheme: row.get("scheme_id")?,
        fields: TileFields {
            kind: named(table, "kind", &kind, TileKind::from_name)?,
            identifier: row.get("identifier")?,
            title: row.get("title")?,
            description: row.get("description")?,
            place: row.get("place")?,
            sources: Vec::new(),
            state,
            external: external_document
                .map(|document| External { document, place: external_place.unwrap_or_default() }),
            sections: sections_from_text(table, &sections)?,
            instructions: row.get("instructions")?,
            x: row.get("x")?,
            y: row.get("y")?,
        },
    })
}

/// The sections of the report of an output, as stored: their names,
/// separated by commas.
pub(in crate::db) fn sections_text(sections: &[OutputSection]) -> String {
    sections.iter().map(|section| section.name()).collect::<Vec<_>>().join(",")
}

/// The sections of an output from their stored text.
fn sections_from_text(table: &'static str, text: &str) -> rusqlite::Result<Vec<OutputSection>> {
    text.split(',')
        .filter(|name| !name.is_empty())
        .map(|name| named(table, "sections", name, OutputSection::from_name))
        .collect()
}

/// A tile of `tiles`, with its state.
pub(super) fn project_tile_from_row(row: &Row<'_>) -> rusqlite::Result<Tile> {
    let state: String = row.get("state")?;

    tile_from_row(row, "tiles", named("tiles", "state", &state, ProposalState::from_name)?)
}

/// Reads the tiles that `tiles_sql` selects (with `:key`), and gives
/// each its sources, which `sources_sql` selects for all of them at
/// once (columns `tile_id, document_id, page, quote, found`, with
/// `:key`): one query for the sources, not one per tile.
pub(in crate::db) fn tiles_with_sources(
    connection: &Connection,
    tiles_sql: &str,
    sources_sql: &str,
    key: i64,
    from_row: impl FnMut(&Row<'_>) -> rusqlite::Result<Tile>,
) -> rusqlite::Result<Vec<Tile>> {
    let mut statement = connection.prepare(tiles_sql)?;
    let mut tiles = statement.query_map(&[(":key", &key)], from_row)?.collect::<Result<Vec<_>, _>>()?;

    let mut statement = connection.prepare(sources_sql)?;
    let rows = statement.query_map(&[(":key", &key)], |row| {
        Ok((
            row.get::<_, i64>("tile_id")?,
            Source {
                document_id: row.get("document_id")?,
                page: row.get("page")?,
                quote: row.get("quote")?,
                found: row.get("found")?,
            },
        ))
    })?;

    let mut sources: HashMap<i64, Vec<Source>> = HashMap::new();

    for row in rows {
        let (tile, source) = row?;
        sources.entry(tile).or_default().push(source);
    }

    for tile in &mut tiles {
        tile.fields.sources = sources.remove(&tile.id).unwrap_or_default();
    }

    Ok(tiles)
}

/// A link from a row of `links`.
pub(super) fn link_from_row(row: &Row<'_>) -> rusqlite::Result<Link> {
    Ok(Link { id: row.get("id")?, from: row.get("from_tile")?, to: row.get("to_tile")? })
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "rows_tests.rs"]
mod tests;
