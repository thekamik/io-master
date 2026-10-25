//! The entry of the component "agent with knowledge" (step 57.6 of
//! docs/todo.md; page: `ui/src/components/knowledge_agent/AgentPicker.svelte`,
//! its command in `ui/src/lib/api/components/knowledge_agent.ts`): the list of
//! the agents a work of the AI may use. Each view keeps which agent its
//! object uses (a master project, a catalog, a chat); the agents themselves
//! are made in the view Knowledge (RAG).

use tauri::State;

use crate::app::error::AppError;
use crate::app::state::AppState;

use super::view::{self as agent_view, AgentOptionView};

/// The agents with knowledge of the evaluator, by name.
///
/// Called by the page: `AgentPicker.svelte`, when it is shown (`knowledgeAgent.knowledgeAgentList()`).
/// Calls: `agent_view::agents`.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn knowledge_agent_list(state: State<'_, AppState>) -> Result<Vec<AgentOptionView>, AppError> {
    let session = state.session()?;

    agent_view::agents(&session.db())
}

#[cfg(test)]
mod tests {
    //! Tests of the command of the component, called as the page calls it.

    use serde_json::json;

    use crate::views::test_support::logged_in_window;

    #[test]
    fn the_agents_are_listed_by_name() {
        let window = logged_in_window();
        assert_eq!(window.call("knowledge_agent_list", json!({})).unwrap(), json!([]));

        window.call("agent_list_create", json!({ "name": "CC agent" })).unwrap();

        assert_eq!(window.call("knowledge_agent_list", json!({})).unwrap()[0]["name"], "CC agent");
    }
}
