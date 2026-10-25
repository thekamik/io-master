<!--
  The element "list of agents" of the view Knowledge (RAG) (its commands
  agent_list_get, agent_list_create and agent_list_delete in
  src/views/knowledge/knowledge.rs): every agent with the number of its
  bases, opened (onopen) or deleted; a new agent, opened at once so that
  its bases are ticked in its form; and an agent imported from a file with
  its bases (agent_list_import, step 57.7), opened once it is stored.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../../lib/api/errors";
  import * as knowledge from "../../lib/api/knowledge";

  let { onopen }: { onopen: (id: number) => void } = $props();

  let agents: knowledge.AgentSummaryView[] = $state([]);
  let name = $state("");
  let making = $state(false);
  let problem = $state("");

  onMount(async () => {
    try {
      agents = await knowledge.agentListGet();
    } catch (error) {
      problem = sentence(error);
    }
  });

  async function make(event: SubmitEvent) {
    event.preventDefault();

    try {
      const made = await knowledge.agentListCreate(name);
      onopen(made.id);
    } catch (error) {
      problem = `The agent was not made: ${sentence(error)}`;
    }
  }

  async function importAgent() {
    try {
      const imported = await knowledge.agentListImport();

      if (imported) onopen(imported.id);
    } catch (error) {
      problem = `The agent was not imported: ${sentence(error)}`;
    }
  }

  async function remove(id: number) {
    try {
      agents = await knowledge.agentListDelete(id);
    } catch (error) {
      problem = sentence(error);
    }
  }
</script>

{#if problem}<p class="problem">{problem}</p>{/if}

{#if making}
  <form class="card block-form" onsubmit={make}>
    <h3>A new agent</h3>
    <p class="note">An agent is a model of your active connection with knowledge: its instructions, the knowledge bases it draws on, and a glossary. You tick its bases in its form, and try it on its proving ground.</p>
    <label>Name <input bind:value={name} placeholder="CC evaluator" required /></label>
    <div class="buttons">
      <button type="submit">Make</button>
      <button type="button" onclick={() => (making = false)}>Cancel</button>
    </div>
  </form>
{:else}
  <div class="buttons lab-actions">
    <button type="button" onclick={() => (making = true)}>New agent…</button>
    <button type="button" onclick={importAgent}>Import an agent…</button>
  </div>
{/if}

{#if agents.length === 0}
  <div class="card">
    <h3>No agent yet</h3>
    <p class="note">Make the knowledge bases first (the tab Knowledge bases), then an agent that draws on them: for example one for Common Criteria as a whole, one for the vulnerability analysis (AVA). Later, the works of the AI in the evaluation only choose an agent.</p>
  </div>
{:else}
  <table class="data">
    <thead><tr><th>Agent</th><th>Knowledge bases</th><th></th></tr></thead>
    <tbody>
      {#each agents as item (item.id)}
        <tr>
          <td>{item.name}</td>
          <td>{item.bases === 0 ? "none yet" : item.bases}</td>
          <td class="actions">
            <button type="button" class="small" onclick={() => onopen(item.id)}>Open</button>
            <button type="button" class="small danger" onclick={() => remove(item.id)}>Delete</button>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}
