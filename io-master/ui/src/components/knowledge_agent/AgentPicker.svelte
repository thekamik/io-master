<!--
  The component "agent with knowledge" (src/components/knowledge_agent/,
  step 57.6): one line to choose the agent a work of the AI uses, none or
  one of the agents made in the view Knowledge (RAG). Shown only when there
  is an agent, so that a window without agents stays as it was. The view
  that shows it keeps the choice (onchange).
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { knowledgeAgentList, type AgentOptionView } from "../../lib/api/components/knowledge_agent";

  let {
    value,
    label = "Agent with knowledge",
    onchange,
  }: { value: number | null; label?: string; onchange: (agent: number | null) => void } = $props();

  let agents: AgentOptionView[] = $state([]);

  onMount(async () => {
    try {
      agents = await knowledgeAgentList();
    } catch {
      // Without the list there is nothing to choose; the line stays hidden.
      agents = [];
    }
  });

  function changed(event: Event) {
    const chosen = (event.currentTarget as HTMLSelectElement).value;

    onchange(chosen === "" ? null : Number(chosen));
  }
</script>

{#if agents.length > 0}
  <label class="inline-choice">
    {label}
    <select value={value === null ? "" : String(value)} onchange={changed}>
      <option value="">none</option>
      {#each agents as agent (agent.id)}<option value={String(agent.id)}>{agent.name}</option>{/each}
    </select>
  </label>
{/if}
