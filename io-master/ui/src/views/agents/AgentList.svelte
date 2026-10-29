<!--
  The tab Agents of the panel of the agents (its commands agent_list and
  agent_create in src/panels/views/agents/agent.rs, integration_list in
  src/panels/views/settings/integrations.rs for the choice): every agent
  with its integration, and a new agent, made with its integration and
  opened at once (onopen).
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { agentCreate, agentList, type AgentSummaryView } from "../../lib/api/agents";
  import { sentence } from "../../lib/api/errors";
  import { integrationList, type IntegrationView } from "../../lib/api/settings";

  let { onopen }: { onopen: (agent: number) => void } = $props();

  let agents: AgentSummaryView[] = $state([]);
  let integrations: IntegrationView[] = $state([]);
  let name = $state("");
  let integration: number | null = $state(null);
  let problem = $state("");
  let busy = $state(false);

  async function reload() {
    try {
      agents = await agentList();
      integrations = await integrationList();
      integration = integrations.find((one) => one.active)?.id ?? integrations[0]?.id ?? null;
    } catch (error) {
      problem = sentence(error);
    }
  }

  async function create(event: SubmitEvent) {
    event.preventDefault();

    if (integration === null) {
      return;
    }

    problem = "";
    busy = true;

    try {
      onopen(await agentCreate(name, integration));
    } catch (error) {
      problem = sentence(error);
    } finally {
      busy = false;
    }
  }

  onMount(reload);
</script>

{#if problem}<p class="problem">{problem}</p>{/if}

<section class="card">
  <h2>Agents</h2>
  <table class="data">
    <thead><tr><th>Agent</th><th>Integration</th><th>Knowledge bases</th><th></th></tr></thead>
    <tbody>
      {#each agents as agent (agent.id)}
        <tr>
          <td class="wrap">{agent.name}</td>
          <td class="wrap">{agent.integration}</td>
          <td>{agent.knowledgeBases}</td>
          <td><button type="button" onclick={() => onopen(agent.id)}>Open</button></td>
        </tr>
      {:else}
        <tr><td colspan="4" class="note">No agent yet: make the first one below.</td></tr>
      {/each}
    </tbody>
  </table>
</section>

<section class="card">
  <h2>A new agent</h2>
  {#if integrations.length === 0}
    <p class="note">An agent asks the model of an integration: add one first in the AI settings.</p>
  {:else}
    <form onsubmit={create}>
      <label>Name <input bind:value={name} required placeholder="agent_WU_1-1" /></label>
      <label>
        Integration
        <span class="note">The model that answers the questions of the agent; it can be changed later.</span>
        <select bind:value={integration}>
          {#each integrations as one (one.id)}
            <option value={one.id}>{one.name} · {one.model}</option>
          {/each}
        </select>
      </label>
      <button type="submit" disabled={busy}>Make the agent and open it</button>
    </form>
  {/if}
</section>

<style>
  table.data {
    width: 100%;
  }
</style>
