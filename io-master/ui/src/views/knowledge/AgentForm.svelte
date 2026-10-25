<!--
  The element "form of an agent" of the view Knowledge (RAG) (its commands
  agent_form_get and agent_form_save in src/views/knowledge/knowledge.rs):
  the name and the instructions of an agent, the knowledge bases it draws
  on (ticked here: the agent is joined to its bases in this one form), its
  glossary and its limits; under it the proving ground (ProvingGround).
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../../lib/api/errors";
  import * as knowledge from "../../lib/api/knowledge";
  import ProvingGround from "./ProvingGround.svelte";

  let { id, onclose }: { id: number; onclose: () => void } = $props();

  let view: knowledge.AgentFormView | null = $state(null);
  let form: knowledge.AgentForm = $state({ name: "", instructions: "", glossary: "", passages: 1, characters: 1, bases: [] });
  let notice: { done: boolean; text: string } | null = $state(null);

  onMount(async () => {
    try {
      show(await knowledge.agentFormGet(id));
    } catch (error) {
      notice = { done: false, text: sentence(error) };
    }
  });

  function show(screen: knowledge.AgentFormView) {
    view = screen;
    form = {
      name: screen.name,
      instructions: screen.instructions,
      glossary: screen.glossary,
      passages: screen.passages,
      characters: screen.characters,
      bases: screen.bases.filter((base) => base.chosen).map((base) => base.id),
    };
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();

    try {
      show(await knowledge.agentFormSave(id, { ...form, bases: [...form.bases] }));
      notice = { done: true, text: "Saved." };
    } catch (error) {
      notice = { done: false, text: `Not saved: ${sentence(error)}` };
    }
  }
</script>

<div class="editor-head">
  <button type="button" class="small" onclick={onclose}>← All agents</button>
</div>

{#if view}
  <form class="card block-form" onsubmit={save}>
    <h3>{view.name}</h3>
    <label>Name <input bind:value={form.name} required /></label>
    <label>Instructions: who the agent is and how it answers <textarea bind:value={form.instructions} rows="4"></textarea></label>

    <div class="knowledge-add">
      <h4>The knowledge bases it draws on</h4>
      {#each view.bases as base (base.id)}
        <label class="choice">
          <input type="checkbox" value={base.id} bind:group={form.bases} />
          {base.name} <span class="note">({base.passages} passages)</span>
        </label>
      {:else}
        <p class="note">No knowledge base yet: make one in the tab Knowledge bases, then tick it here.</p>
      {/each}
    </div>

    <div class="knowledge-add">
      <h4>Glossary</h4>
      <p class="note">One entry per line, <code>term = other term</code>: when a question holds one side, the search looks for the other too. A line that starts with # is a comment.</p>
      <textarea bind:value={form.glossary} rows="5" placeholder="TSF = TOE Security Functionality&#10;ST = Security Target"></textarea>
    </div>

    <div class="knowledge-add">
      <h4>What it gives the model with a question</h4>
      <label>Passages, at most <input type="number" min="1" max={view.max_passages} bind:value={form.passages} /></label>
      <label>Characters of the passages, at most <input type="number" min={view.min_characters} max={view.max_characters} step="500" bind:value={form.characters} /></label>
    </div>

    <div class="buttons"><button type="submit">Save</button></div>
    {#if notice}<p class={notice.done ? "done" : "problem"}>{notice.text}</p>{/if}
  </form>

  <ProvingGround agent={id} />
{:else if notice}
  <p class="problem">{notice.text}</p>
{/if}
