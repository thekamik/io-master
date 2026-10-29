<!--
  The tab Knowledge bases of the panel of the agents (its commands
  knowledge_base_list and knowledge_base_create in
  src/panels/views/agents/knowledge.rs): every base, shared by the agents
  that name it, with its sources counted, and a new base, opened at once in
  its editor (onopen).
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { knowledgeBaseCreate, knowledgeBaseList, type BaseView } from "../../lib/api/agents";
  import { sentence } from "../../lib/api/errors";

  let { onopen }: { onopen: (base: number) => void } = $props();

  let bases: BaseView[] = $state([]);
  let name = $state("");
  let description = $state("");
  let problem = $state("");
  let busy = $state(false);

  async function reload() {
    try {
      bases = await knowledgeBaseList();
    } catch (error) {
      problem = sentence(error);
    }
  }

  async function create(event: SubmitEvent) {
    event.preventDefault();
    problem = "";
    busy = true;

    try {
      onopen(await knowledgeBaseCreate(name, description));
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
  <h2>Knowledge bases</h2>
  <table class="data">
    <thead><tr><th>Base</th><th>Description</th><th>Sources</th><th></th></tr></thead>
    <tbody>
      {#each bases as base (base.id)}
        <tr>
          <td class="wrap">{base.name}</td>
          <td class="wrap">{base.description}</td>
          <td>{base.sources}</td>
          <td><button type="button" onclick={() => onopen(base.id)}>Open</button></td>
        </tr>
      {:else}
        <tr><td colspan="4" class="note">No knowledge base yet: make the first one below.</td></tr>
      {/each}
    </tbody>
  </table>
</section>

<section class="card">
  <h2>A new knowledge base</h2>
  <form onsubmit={create}>
    <label>Name <input bind:value={name} required placeholder="Earlier evaluations" /></label>
    <label>Description <input bind:value={description} /></label>
    <button type="submit" disabled={busy}>Make the base and open it</button>
  </form>
</section>

<style>
  table.data {
    width: 100%;
  }
</style>
