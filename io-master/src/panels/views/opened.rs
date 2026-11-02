//! The open database of the user logged in, as every command of a panel
//! takes it from the session: the lock of the session held for the whole
//! command, so that two commands use the database one after the other.
//!
//! ```text
//! let opened = session.lock()?;        the lock, for the command
//! let db = database(opened.as_ref())?; the database, or "nobody is logged in"
//! ```

use crate::infrastructure::storage::user_database::UserDatabase;
use crate::panels::views::error::ViewError;

/// The open database of the session `opened`.
///
/// Input: `opened`, what the guard of `panels/session.rs`: `Session::lock` holds.
/// Output: the database; `ViewError::NotLoggedIn` when nobody is logged in.
///
/// Called by: `panels/views/agents/agent.rs`: `agent_list`, `agent_create`, `agent_get`, `agent_save`,
/// `agent_knowledge_test`; `panels/views/agents/journal.rs`: `agent_journal_list`,
/// `agent_journal_get`; `panels/views/agents/knowledge.rs`: `knowledge_base_list`,
/// `knowledge_base_create`, `knowledge_base_save`, `knowledge_source_list`, `knowledge_source_add`,
/// `knowledge_source_replace`, `knowledge_source_rename`, `knowledge_search`;
/// `panels/views/chat/chats.rs`: `chat_list`, `chat_create`, `chat_get`, `chat_rename`,
/// `prepared_question`, `written`; `panels/views/evaluation/evaluations.rs`: `evaluation_list`,
/// `evaluation_create`, `evaluation_get`, `evaluation_set_agent`;
/// `panels/views/evaluation/inputs.rs`: `evaluation_document_give`, `evaluation_proof_give`;
/// `panels/views/evaluation/run.rs`: `evaluation_assess_preview`, `evaluation_assess`;
/// `panels/views/evaluation/work_units.rs`: `evaluation_unit_list`, `evaluation_unit_get`,
/// `evaluation_unit_save`; `panels/views/evaluation_plan/fill.rs`: `fill`, `plan_fill_preview`;
/// `panels/views/evaluation_plan/plans.rs`: `plan_list`, `plan_create`, `plan_get`, `plan_save`,
/// `plan_add_document`; `panels/views/evaluation_plan/standards.rs`: `standard_list`,
/// `standard_import`; `panels/views/evaluation_plan/work_units.rs`: `work_unit_list`,
/// `work_unit_controls`, `work_unit_add`, `work_unit_add_all`, `work_unit_remove`, `work_unit_get`,
/// `work_unit_save`, `work_unit_add_proof`; `panels/views/settings/integrations.rs`:
/// `integration_list`, `integration_create`, `integration_save`, `integration_activate`,
/// `integration_set_key`, `integration_check`.
pub fn database(opened: Option<&UserDatabase>) -> Result<&UserDatabase, ViewError> {
    match opened {
        Some(database) => Ok(database),
        None => Err(ViewError::NotLoggedIn),
    }
}
