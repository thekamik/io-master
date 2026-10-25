//! The export of an agent with knowledge (step 57.7 of docs/todo.md): one
//! JSON file with the agent and its knowledge bases, their sources and
//! every passage, so that a colleague imports it whole (`agent_list/import.rs`).
//! Not the whole database: a file taken by someone holds this agent only
//! (user, 2026-10-25). The file holds the text of the sources, which may be
//! licensed standards; the window says so before it is written.
//!
//! ```text
//! { "format": "io-master-agent", "version": 1, "exported": <seconds since 1970>,
//!   "agent": { name, instructions, glossary, passages, characters },
//!   "bases": [ { name, description,
//!                sources: [ { kind, name, passages: [ { page, label, text } ] } ] } ] }
//! ```

use serde_json::{Value, json};

use crate::app::error::AppError;
use crate::components::knowledge_agent::find;
use crate::db::knowledge as db_knowledge;
use crate::db::user::database::UserDb;
use crate::model::time::now;

/// The name of the format, written into every file.
pub const FORMAT: &str = "io-master-agent";

/// The version of the format.
pub const FORMAT_VERSION: u64 = 1;

/// The name of agent `id` and the text of its file.
///
/// Called by: `agent_form_export` (`knowledge.rs`).
/// Calls: `find::agent`, [`base_json`].
pub fn file_text(db: &UserDb, id: i64) -> Result<(String, String), AppError> {
    let agent = find::agent(db, id)?;
    let mut bases = Vec::with_capacity(agent.fields.bases.len());

    for base in &agent.fields.bases {
        bases.push(base_json(db, *base)?);
    }

    let file = json!({
        "format": FORMAT,
        "version": FORMAT_VERSION,
        "exported": now(),
        "agent": {
            "name": agent.fields.name,
            "instructions": agent.fields.instructions,
            "glossary": agent.fields.glossary,
            "passages": agent.fields.passages,
            "characters": agent.fields.characters,
        },
        "bases": bases,
    });
    let text = serde_json::to_string(&file).map_err(|error| AppError::Failed(error.to_string()))?;

    Ok((agent.fields.name, text))
}

/// Base `id` as the file writes it, with its sources and their passages.
///
/// Called by: [`file_text`].
/// Calls: `db_knowledge::read::base`, `read::sources`, `read::passages_of`.
fn base_json(db: &UserDb, id: i64) -> Result<Value, AppError> {
    let Some(base) = db_knowledge::read::base(db, id)? else {
        return Err(AppError::Invalid("a knowledge base of the agent does not exist".to_owned()));
    };
    let stored = db_knowledge::read::sources(db, id)?;
    let mut sources = Vec::with_capacity(stored.len());

    for source in stored {
        let mut passages = Vec::with_capacity(source.passages);

        for passage in db_knowledge::read::passages_of(db, source.id)? {
            passages.push(json!({ "page": passage.page, "label": passage.label, "text": passage.text }));
        }

        sources.push(json!({ "kind": source.kind.name(), "name": source.name, "passages": passages }));
    }

    Ok(json!({ "name": base.name, "description": base.description, "sources": sources }))
}
