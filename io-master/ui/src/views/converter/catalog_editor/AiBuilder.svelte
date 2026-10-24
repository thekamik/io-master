<!--
  The element "the AI" of the editor of a catalog
  (its commands in src/views/converter/catalog_editor/catalog_editor.rs): the AI reads a table of contents
  and proposes the groups, or the pages of a group (the group chosen in
  the tree, else the top) and proposes its controls. Its events arrive
  while it works, and "Stop" ends it after its step. When it is over it
  tells the view (onchanged): the proposals wait in their element.
  "Instructions" only tell the view, which opens their dialog.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../../../lib/api/errors";
  import * as catalogEditor from "../../../lib/api/converter/catalog_editor";
  import type { Which } from "../../../lib/api/converter/catalog_editor";
  import { applyBuildEvent } from "./build_progress";

  let {
    uuid,
    group,
    onchanged,
    oninstructions,
  }: {
    uuid: string;
    group: string | null;
    onchanged: () => void;
    oninstructions: (which: Which) => void;
  } = $props();

  let sources: catalogEditor.SourceView[] = $state([]);
  let outline = $state({ document_id: 0, first_page: 1, last_page: 1 });
  let section = $state({ document_id: 0, first_page: 1, last_page: 1, partSize: 12000 });
  let building = $state(false);
  let progress = $state("");
  let log: { done: boolean; text: string }[] = $state([]);
  let problem = $state("");

  onMount(async () => {
    try {
      sources = await catalogEditor.aiBuilderSources();
      outline.document_id = section.document_id = sources[0]?.id ?? 0;
    } catch (error) {
      problem = sentence(error);
    }
  });

  function buildEvent(event: catalogEditor.BuildEvent) {
    const state = { progress, log };
    applyBuildEvent(state, event);
    progress = state.progress;
  }

  async function build(work: () => Promise<void>) {
    building = true;
    log = [];
    progress = problem = "";

    try {
      await work();
    } catch (error) {
      problem = sentence(error);
    } finally {
      building = false;
      onchanged();
    }
  }

  function buildGroups(event: SubmitEvent) {
    event.preventDefault();
    const range = { ...outline };
    build(() => catalogEditor.aiBuilderGroups(uuid, range, buildEvent));
  }

  function buildControls(event: SubmitEvent) {
    event.preventDefault();
    const { partSize, ...range } = section;
    build(() => catalogEditor.aiBuilderControls(uuid, group, [range], partSize, buildEvent));
  }
</script>

<h3>The AI</h3>
{#if problem}<p class="problem">{problem}</p>{/if}
{#if sources.length === 0}
  <p class="note">Load the PDFs of the standard (tab Documents of standards) for the AI to read them.</p>
{:else}
  <form onsubmit={buildGroups}>
    <h4>Groups from the table of contents</h4>
    <select bind:value={outline.document_id}>
      {#each sources as source (source.id)}<option value={source.id}>{source.title}</option>{/each}
    </select>
    <div class="range">
      <label>Pages <input type="number" min="1" bind:value={outline.first_page} /></label>
      <label>to <input type="number" min="1" bind:value={outline.last_page} /></label>
    </div>
    <button type="submit" disabled={building}>Propose the groups</button>
  </form>

  <form onsubmit={buildControls}>
    <h4>Controls of {group ? `group ${group}` : "the top of the catalog"}</h4>
    <select bind:value={section.document_id}>
      {#each sources as source (source.id)}<option value={source.id}>{source.title}</option>{/each}
    </select>
    <div class="range">
      <label>Pages <input type="number" min="1" bind:value={section.first_page} /></label>
      <label>to <input type="number" min="1" bind:value={section.last_page} /></label>
    </div>
    <label>Characters per question <input type="number" min="2000" step="1000" bind:value={section.partSize} /></label>
    <button type="submit" disabled={building}>Propose the controls</button>
  </form>

  <div class="buttons">
    <button type="button" class="small" onclick={() => oninstructions("catalog_outline")}>Instructions: groups</button>
    <button type="button" class="small" onclick={() => oninstructions("catalog_section")}>Instructions: controls</button>
  </div>
{/if}

{#if building || progress || log.length > 0}
  <div class="load-log">
    {#if progress}<p class="note">{progress}</p>{/if}
    {#each log as line, index (index)}
      <div class={line.done ? "done" : "problem"}>{line.text}</div>
    {/each}
    {#if building}
      <button type="button" class="small danger" onclick={catalogEditor.aiBuilderStop}>Stop</button>
    {:else}
      <button type="button" class="small" onclick={() => (log = [])}>Close</button>
    {/if}
  </div>
{/if}
