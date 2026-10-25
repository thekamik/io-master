<!--
  The element "list of knowledge bases" of the view Knowledge (RAG) (its
  commands knowledge_list_get, knowledge_list_create and
  knowledge_list_delete in src/views/knowledge/knowledge.rs): every base
  with the numbers of its sources and passages, opened (onopen) or deleted;
  and a new base, opened once it is made.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../../lib/api/errors";
  import * as knowledge from "../../lib/api/knowledge";

  let { onopen }: { onopen: (id: number) => void } = $props();

  let bases: knowledge.BaseSummaryView[] = $state([]);
  let form: knowledge.BaseForm = $state({ name: "", description: "" });
  let making = $state(false);
  let problem = $state("");

  onMount(async () => {
    try {
      bases = await knowledge.knowledgeListGet();
    } catch (error) {
      problem = sentence(error);
    }
  });

  async function make(event: SubmitEvent) {
    event.preventDefault();

    try {
      const made = await knowledge.knowledgeListCreate({ ...form });
      onopen(made.id);
    } catch (error) {
      problem = `The knowledge base was not made: ${sentence(error)}`;
    }
  }

  async function remove(id: number) {
    try {
      bases = await knowledge.knowledgeListDelete(id);
    } catch (error) {
      problem = sentence(error);
    }
  }
</script>

{#if problem}<p class="problem">{problem}</p>{/if}

{#if making}
  <form class="card block-form" onsubmit={make}>
    <h3>A new knowledge base</h3>
    <p class="note">A base holds the sources an agent of the AI may draw on: the parts of a standard, its methodology, the notes of the scheme or of the laboratory. Make one per standard or per kind of task.</p>
    <label>Name <input bind:value={form.name} placeholder="Common Criteria 2022" required /></label>
    <label>What it is for <textarea bind:value={form.description} rows="2" placeholder="CC Part 1 to 5 and the CEM"></textarea></label>
    <div class="buttons">
      <button type="submit">Make</button>
      <button type="button" onclick={() => (making = false)}>Cancel</button>
    </div>
  </form>
{:else}
  <div class="buttons lab-actions"><button type="button" onclick={() => (making = true)}>New knowledge base…</button></div>
{/if}

{#if bases.length === 0}
  <div class="card">
    <h3>No knowledge base yet</h3>
    <p class="note">The model of a connection knows a standard only from what it learned, and may remember it wrongly. A knowledge base gives it the text itself: before a question, the program finds the passages that fit it and puts them into the question with their source and page. The model is not trained; you choose what it reads, and every passage it gets can be seen.</p>
  </div>
{:else}
  <table class="data">
    <thead><tr><th>Knowledge base</th><th>What it is for</th><th>Sources</th><th>Passages</th><th></th></tr></thead>
    <tbody>
      {#each bases as item (item.id)}
        <tr>
          <td>{item.name}</td>
          <td>{item.description}</td>
          <td>{item.sources}</td>
          <td>{item.passages}</td>
          <td class="actions">
            <button type="button" class="small" onclick={() => onopen(item.id)}>Open</button>
            <button type="button" class="small danger" onclick={() => remove(item.id)}>Delete</button>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}
