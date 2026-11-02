//! The knowledge bases of the panel of the agents, as the page calls them:
//! the bases, a new one, one saved, their sources (a file chosen in the
//! dialog of the system; the page never sees a path), the text of a source
//! replaced by a newer file, a source renamed, the search in a base.
//!
//! ```text
//! KnowledgeBases.svelte ──knowledge_base_list()───────► the bases, their sources counted ──► [BaseView]
//!                       ──knowledge_base_create(...)──► add_base ──► id
//!                       ──knowledge_base_save(...)────► change_base
//! BaseEditor.svelte     ──knowledge_source_list(base)─► read_sources ──► [SourceView]
//!                       ──knowledge_source_add(base)──► the dialog ──► source_text ──► add_source ──► id
//!                       ──knowledge_source_replace(id)► the dialog ──► source_text ──► replace_source_text
//!                       ──knowledge_source_rename()───► rename_source
//!                       ──knowledge_search(base, q)───► search_base ──► the passages found
//! ```

use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use crate::objects::ai::rag::rag_model::KnowledgeBase;
use crate::objects::ai::rag::rag_rules::{
    add_base, add_source, change_base, rename_source, replace_source_text, search_base, source_text,
};
use crate::objects::ai::rag::rag_store::{read_knowledge_bases, read_sources};
use crate::panels::session::Session;
use crate::panels::views::error::ViewError;
use crate::panels::views::opened::database;

/// The longest name, description or question from the page, in bytes.
const MAX_FIELD: usize = 8192;

/// A knowledge base, as the list shows it.
#[derive(Serialize)]
pub struct BaseView {
    id: i64,
    name: String,
    description: String,
    sources: usize,
}

/// A source of a base, without its text.
#[derive(Serialize)]
pub struct SourceView {
    id: i64,
    name: String,
    characters: usize,
}

/// Every knowledge base, by its name, its sources counted.
///
/// Input: `session`, the session of the window, someone logged in.
/// Output: the bases (`[{"id": 1, "name": "Common Criteria", "sources": 4}]`);
/// `ViewError::NotLoggedIn` when nobody is logged in, an error of the database.
///
/// Called by: `ui/src/lib/api/agents.ts`: `knowledgeBaseList`, from `ui/src/views/agents/KnowledgeBases.svelte`,
/// when the tab opens and after a change.
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/ai/rag/rag_store.rs`: `read_knowledge_bases`, `read_sources`.
#[tauri::command]
pub fn knowledge_base_list(session: State<'_, Session>) -> Result<Vec<BaseView>, ViewError> {
    let opened = session.lock()?;
    let db = database(opened.as_ref())?;
    let mut bases = Vec::new();

    for base in read_knowledge_bases(db)? {
        let sources = read_sources(db, base.id)?.len();

        bases.push(BaseView { id: base.id, name: base.name, description: base.description, sources });
    }

    Ok(bases)
}

/// Makes a new knowledge base; gives its id.
///
/// Input: `session`, the session of the window, someone logged in; `name`,
/// `description`, as typed.
/// Output: the id of the new base (`4`); `ViewError::NotLoggedIn` when nobody
/// is logged in, the refusals of the base.
///
/// Called by: `ui/src/lib/api/agents.ts`: `knowledgeBaseCreate`, from `ui/src/views/agents/KnowledgeBases.svelte`,
/// "Make the base".
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/ai/rag/rag_rules.rs`: `add_base`.
#[tauri::command]
pub fn knowledge_base_create(session: State<'_, Session>, name: String, description: String) -> Result<i64, ViewError> {
    ensure!(name.len() + description.len() <= MAX_FIELD, ViewError::Invalid("the text is too long".to_owned()));

    let opened = session.lock()?;

    Ok(add_base(database(opened.as_ref())?, &name, &description)?)
}

/// Saves the name and the description of the base `id`.
///
/// Input: `session`, the session of the window, someone logged in; `id`, the id
/// of a base; `name`, `description`, as typed.
/// Output: nothing; `ViewError::NotLoggedIn` when nobody is logged in, the
/// refusals of the base.
///
/// Called by: `ui/src/lib/api/agents.ts`: `knowledgeBaseSave`, from `ui/src/views/agents/BaseEditor.svelte`,
/// "Save".
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/ai/rag/rag_rules.rs`: `change_base`.
#[tauri::command]
pub fn knowledge_base_save(
    session: State<'_, Session>,
    id: i64,
    name: String,
    description: String,
) -> Result<(), ViewError> {
    ensure!(name.len() + description.len() <= MAX_FIELD, ViewError::Invalid("the text is too long".to_owned()));

    let opened = session.lock()?;

    Ok(change_base(database(opened.as_ref())?, &KnowledgeBase { id, name, description })?)
}

/// The sources of the base `base`, by their names, without their texts.
///
/// Input: `session`, the session of the window, someone logged in; `base`, the
/// id of a base.
/// Output: the sources (`[{"id": 12, "name": "CEM.pdf", "characters": 1200000}]`);
/// `ViewError::NotLoggedIn` when nobody is logged in, an error of the database.
///
/// Called by: `ui/src/lib/api/agents.ts`: `knowledgeSourceList`, from `ui/src/views/agents/BaseEditor.svelte`, when
/// a base is opened and after a change.
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/ai/rag/rag_store.rs`: `read_sources`.
#[tauri::command]
pub fn knowledge_source_list(session: State<'_, Session>, base: i64) -> Result<Vec<SourceView>, ViewError> {
    let opened = session.lock()?;
    let mut sources = Vec::new();

    for source in read_sources(database(opened.as_ref())?, base)? {
        sources.push(SourceView { id: source.id, name: source.name, characters: source.text.chars().count() });
    }

    Ok(sources)
}

/// Opens the dialog of the system for a file of a source; gives the name of
/// the file and its text, or `None` when the dialog was closed.
///
/// Input: `app`, the handle of the window; `title`, the title of the dialog.
/// Output: the name of the file and its text (`Some(("CEM.pdf", "..."))`),
/// `None` when the dialog was closed; `ViewError::Invalid` for a file
/// refused.
///
/// Called by: [`knowledge_source_add`], [`knowledge_source_replace`].
/// Calls: the dialog of the system; `objects/ai/rag/rag_rules.rs`: `source_text`.
fn chosen_text(app: &AppHandle, title: &str) -> Result<Option<(String, String)>, ViewError> {
    let Some(file) = app
        .dialog()
        .file()
        .set_title(title)
        .add_filter("Text, Markdown or PDF", &["txt", "md", "pdf"])
        .blocking_pick_file()
    else {
        return Ok(None);
    };
    let path = file.into_path().map_err(|error| ViewError::Invalid(format!("the file chosen: {error}")))?;
    let name = path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();

    Ok(Some((name, source_text(&path)?)))
}

/// Opens the dialog of the system for a file and adds its text to the base
/// `base` as a new source named as the file; gives its id, `None` when the
/// dialog was closed. Run apart from the window (`async`): the dialog waits
/// for the user, and a PDF takes a moment.
///
/// Input: `app`, the handle of the window; `session`, the session of the
/// window, someone logged in; `base`, the id of a base.
/// Output: the id of the new source (`Some(12)`), `None` when the dialog was
/// closed; `ViewError::NotLoggedIn` when nobody is logged in, the refusals of
/// the file or the source.
///
/// Called by: `ui/src/lib/api/agents.ts`: `knowledgeSourceAdd`, from `ui/src/views/agents/BaseEditor.svelte`, "Add
/// a file".
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`; [`chosen_text`];
/// `objects/ai/rag/rag_rules.rs`: `add_source`.
#[tauri::command(async)]
pub fn knowledge_source_add(app: AppHandle, session: State<'_, Session>, base: i64) -> Result<Option<i64>, ViewError> {
    let Some((name, text)) = chosen_text(&app, "Add a source")? else {
        return Ok(None);
    };
    let opened = session.lock()?;

    Ok(Some(add_source(database(opened.as_ref())?, base, &name, text)?))
}

/// Opens the dialog of the system for a newer file of the source `id` and
/// replaces its text; gives whether a file was chosen.
///
/// Input: `app`, the handle of the window; `session`, the session of the
/// window, someone logged in; `id`, the id of a source.
/// Output: whether a file was chosen (`true`); `ViewError::NotLoggedIn` when
/// nobody is logged in, the refusals of the file, an error of the database.
///
/// Called by: `ui/src/lib/api/agents.ts`: `knowledgeSourceReplace`, from `ui/src/views/agents/BaseEditor.svelte`,
/// "Replace the text".
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`; [`chosen_text`];
/// `objects/ai/rag/rag_rules.rs`: `replace_source_text`.
#[tauri::command(async)]
pub fn knowledge_source_replace(app: AppHandle, session: State<'_, Session>, id: i64) -> Result<bool, ViewError> {
    let Some((_, text)) = chosen_text(&app, "Replace the text of a source")? else {
        return Ok(false);
    };
    let opened = session.lock()?;

    replace_source_text(database(opened.as_ref())?, id, text)?;

    Ok(true)
}

/// Changes the name of the source `id`.
///
/// Input: `session`, the session of the window, someone logged in; `id`, the id
/// of a source; `name`, the name typed.
/// Output: nothing; `ViewError::NotLoggedIn` when nobody is logged in, the
/// refusals of the name.
///
/// Called by: `ui/src/lib/api/agents.ts`: `knowledgeSourceRename`, from `ui/src/views/agents/BaseEditor.svelte`,
/// "Rename".
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/ai/rag/rag_rules.rs`: `rename_source`.
#[tauri::command]
pub fn knowledge_source_rename(session: State<'_, Session>, id: i64, name: String) -> Result<(), ViewError> {
    ensure!(name.len() <= MAX_FIELD, ViewError::Invalid("the name is too long".to_owned()));

    let opened = session.lock()?;

    Ok(rename_source(database(opened.as_ref())?, id, &name)?)
}

/// The passages of the base `base` that fit `question` best.
///
/// Input: `session`, the session of the window, someone logged in; `base`, the
/// id of a base; `question`, the question typed.
/// Output: the passages found, the best first (`["..."]`);
/// `ViewError::NotLoggedIn` when nobody is logged in, an error of the database.
///
/// Called by: `ui/src/lib/api/agents.ts`: `knowledgeSearch`, from `ui/src/views/agents/BaseEditor.svelte`,
/// "Search".
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/ai/rag/rag_rules.rs`: `search_base`.
#[tauri::command]
pub fn knowledge_search(session: State<'_, Session>, base: i64, question: String) -> Result<Vec<String>, ViewError> {
    ensure!(question.len() <= MAX_FIELD, ViewError::Invalid("the question is too long".to_owned()));

    let opened = session.lock()?;

    Ok(search_base(database(opened.as_ref())?, base, &question)?)
}
