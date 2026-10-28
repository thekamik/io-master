//! The import of an agent with knowledge (step 57.7 of docs/todo.md): the
//! file of `agent_form/export.rs` read and checked against every limit of
//! the program (the numbers of bases, sources and passages, the length of
//! every text) before anything is stored; then the agent and its bases are
//! stored in one transaction, under free names (`db/knowledge/create.rs`).

use std::path::Path;

use serde_json::Value;

use crate::app::error::AppError;
use crate::model_old::knowledge::{
    AGENT_FILE_MAX, AgentFields, MAX_BASES, MAX_PASSAGES, MAX_SOURCES, NAME_MAX, NewBase, NewSource, PASSAGE_CHARS,
    Passage, SourceKind, check_agent, check_base,
};
use crate::model_old::text::{Field, check_text};

use super::super::agent_form::export::{FORMAT, FORMAT_VERSION};

/// The text of the file at `path`, refused when it is larger than
/// `AGENT_FILE_MAX`.
///
/// Called by: `agent_list_import` (`knowledge.rs`), on a thread for blocking work.
pub fn read_file(path: &Path) -> Result<String, AppError> {
    let size =
        std::fs::metadata(path).map_err(|error| AppError::Failed(format!("the file was not read: {error}")))?.len();

    ensure!(size <= AGENT_FILE_MAX, AppError::Invalid("the file is too large for an agent".to_owned()));

    std::fs::read_to_string(path).map_err(|error| AppError::Invalid(format!("the file is no text: {error}")))
}

/// The agent and its bases in the text of a file; refused when it is no
/// file of an agent of this version or breaks a limit.
///
/// Called by: `agent_list_import` (`knowledge.rs`).
/// Calls: [`agent_of`], [`base_of`].
pub fn read(text: &str) -> Result<(AgentFields, Vec<NewBase>), AppError> {
    let value: Value =
        serde_json::from_str(text).map_err(|error| AppError::Invalid(format!("the file is no JSON: {error}")))?;

    ensure!(
        value.get("format").and_then(Value::as_str) == Some(FORMAT),
        AppError::Invalid("the file is no agent of IO Master".to_owned())
    );
    ensure!(
        value.get("version").and_then(Value::as_u64) == Some(FORMAT_VERSION),
        AppError::Invalid("the file is of another version of the format".to_owned())
    );

    let fields = agent_of(value.get("agent").unwrap_or(&Value::Null))?;
    let items = array(&value, "bases", MAX_BASES)?;
    let mut bases = Vec::with_capacity(items.len());

    for item in items {
        bases.push(base_of(item)?);
    }

    Ok((fields, bases))
}

/// The items of the array `name` of `value` (none when it is missing);
/// refused when there are more than `max`.
///
/// Called by: [`read`], [`base_of`], [`source_of`].
fn array<'a>(value: &'a Value, name: &str, max: usize) -> Result<&'a [Value], AppError> {
    let items = match value.get(name).and_then(Value::as_array) {
        Some(items) => items.as_slice(),
        None => &[],
    };

    ensure!(items.len() <= max, AppError::Invalid(format!("the file has too many {name}")));

    Ok(items)
}

/// The text `name` of `value`; empty when it is missing.
fn text(value: &Value, name: &str) -> String {
    value.get(name).and_then(Value::as_str).unwrap_or_default().to_owned()
}

/// A number `name` of `value`; 0 when it is missing (which the checks refuse).
fn number(value: &Value, name: &str) -> usize {
    value.get(name).and_then(Value::as_u64).and_then(|number| usize::try_from(number).ok()).unwrap_or(0)
}

/// The fields of the agent of the file, checked; its bases are set when
/// they are stored.
///
/// Called by: [`read`].
/// Calls: `check_agent`.
fn agent_of(value: &Value) -> Result<AgentFields, AppError> {
    let fields = AgentFields {
        name: text(value, "name").trim().to_owned(),
        instructions: text(value, "instructions"),
        glossary: text(value, "glossary"),
        passages: number(value, "passages"),
        characters: number(value, "characters"),
        bases: Vec::new(),
    };

    check_agent(&fields).map_err(|error| AppError::Invalid(format!("the agent of the file: {error}")))?;

    Ok(fields)
}

/// A base of the file with its sources, checked.
///
/// Called by: [`read`].
/// Calls: `check_base`, [`source_of`].
fn base_of(value: &Value) -> Result<NewBase, AppError> {
    let name = text(value, "name").trim().to_owned();
    let description = text(value, "description");

    check_base(&name, &description).map_err(|error| AppError::Invalid(format!("a base of the file: {error}")))?;

    let items = array(value, "sources", MAX_SOURCES)?;
    let mut sources = Vec::with_capacity(items.len());

    for item in items {
        sources.push(source_of(item)?);
    }

    Ok(NewBase { name, description, sources })
}

/// A source of the file with its passages, checked: a known kind, a name,
/// passages of text within `PASSAGE_CHARS`.
///
/// Called by: [`base_of`].
fn source_of(value: &Value) -> Result<NewSource, AppError> {
    let Some(kind) = SourceKind::from_name(&text(value, "kind")) else {
        return Err(AppError::Invalid("a source of the file has no known kind".to_owned()));
    };
    let name = text(value, "name").trim().to_owned();
    check_text(Field::Name, &name, NAME_MAX, false).map_err(|error| AppError::Invalid(error.to_string()))?;

    let items = array(value, "passages", MAX_PASSAGES)?;
    let mut passages = Vec::with_capacity(items.len());

    for item in items {
        let passage = Passage {
            page: item.get("page").and_then(Value::as_u64).and_then(|page| u32::try_from(page).ok()),
            label: text(item, "label"),
            text: text(item, "text"),
        };

        ensure!(
            !passage.text.trim().is_empty() && passage.text.chars().count() <= PASSAGE_CHARS,
            AppError::Invalid(format!("a passage of the file is empty or longer than {PASSAGE_CHARS} characters"))
        );
        check_text(Field::Name, &passage.label, NAME_MAX, false)
            .map_err(|error| AppError::Invalid(error.to_string()))?;
        passages.push(passage);
    }

    Ok(NewSource { kind, name, passages })
}

#[cfg(test)]
mod tests {
    //! Unit tests of `import.rs`: a file exported by `agent_form/export.rs`
    //! read back whole, and files that break the format or a limit.

    use super::*;
    use crate::db_old::knowledge as db_knowledge;
    use crate::test_support::standards::database;
    use crate::views::knowledge::agent_form::export::file_text;

    #[test]
    fn an_exported_agent_is_imported_with_its_bases_and_passages() {
        let test = database();
        let agent = crate::test_support::db::knowledge_agent(&test.db, "Roles are allocated by the management.");
        let (name, text) = file_text(&test.db, agent).unwrap();
        assert_eq!(name, "ISO agent");

        let (fields, bases) = read(&text).unwrap();
        assert_eq!((fields.name.as_str(), bases.len()), ("ISO agent", 1));
        assert_eq!(bases[0].sources[0].passages[0].page, Some(7));

        let id = db_knowledge::create::imported_agent(&test.db, &fields, &bases).unwrap();
        let imported = db_knowledge::read::agent(&test.db, id).unwrap().unwrap();
        assert_eq!(imported.fields.name, "ISO agent (imported)");
        let hits = db_knowledge::read::search_agent(&test.db, id, "\"management\"", 5).unwrap();
        assert_eq!(hits.len(), 1, "the passages are indexed again");
    }

    #[test]
    fn a_file_of_another_format_or_too_long_a_passage_is_refused() {
        assert!(read("{}").is_err());
        assert!(read("not JSON").is_err());

        let agent =
            serde_json::json!({ "name": "A", "instructions": "", "glossary": "", "passages": 4, "characters": 4000 });
        let passage = serde_json::json!({ "page": 1, "label": "", "text": "x".repeat(PASSAGE_CHARS + 1) });
        let file = serde_json::json!({
            "format": FORMAT, "version": FORMAT_VERSION, "agent": agent,
            "bases": [{ "name": "B", "description": "", "sources": [{ "kind": "pdf", "name": "S", "passages": [passage] }] }],
        });
        assert!(read(&file.to_string()).is_err());

        let wrong_agent = serde_json::json!({ "format": FORMAT, "version": FORMAT_VERSION, "agent": { "name": "A" } });
        assert!(read(&wrong_agent.to_string()).is_err(), "an agent without its limits");
    }
}
