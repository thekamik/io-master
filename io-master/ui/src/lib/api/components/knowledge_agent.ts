// The command of the component "agent with knowledge"
// (src/components/knowledge_agent/knowledge_agent.rs, step 57.6): the agents
// a work of the AI may use.

import { invoke } from "@tauri-apps/api/core";

// view.rs
/** An agent with knowledge that may be chosen. */
export interface AgentOptionView {
  id: number;
  name: string;
}

/** Called by AgentPicker.svelte, when it is shown. */
export function knowledgeAgentList(): Promise<AgentOptionView[]> {
  return invoke("knowledge_agent_list");
}
