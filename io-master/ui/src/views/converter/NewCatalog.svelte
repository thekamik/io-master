<!--
  The element "new catalog" of the view Converter
  (its commands in src/views/converter/converter.rs): a dialog that makes an empty catalog
  from documents of a standard. When it is made it tells the view
  (onmade), which opens it in its editor.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../../lib/api/errors";
  import * as converter from "../../lib/api/converter";

  let { onmade, onclose }: { onmade: (uuid: string) => void; onclose: () => void } = $props();

  let sources: converter.SourceView[] = $state([]);
  let title = $state("");
  let version = $state("");
  let chosen: number[] = $state([]);
  let problem = $state("");

  onMount(async () => {
    try {
      sources = await converter.newCatalogSources();
    } catch (error) {
      problem = sentence(error);
    }
  });

  async function make(event: SubmitEvent) {
    event.preventDefault();

    try {
      const made = await converter.newCatalogCreate(title, version, chosen);
      onmade(made.uuid);
    } catch (error) {
      problem = `The catalog was not made: ${sentence(error)}`;
    }
  }
</script>

<div class="overlay" role="presentation">
  <form class="dialog" onsubmit={make}>
    <h2>A new catalog</h2>
    <p class="note">
      A standard, its methodology (the CEM), a Protection Profile or a test specification: each is a catalog of its own.
    </p>
    {#if problem}<p class="problem">{problem}</p>{/if}
    <label>Title <input bind:value={title} required placeholder="Common Criteria Part 2: Security functional components" /></label>
    <label>Version <input bind:value={version} placeholder="CC:2022 Revision 1" /></label>
    <fieldset>
      <legend>Comes from the documents</legend>
      {#each sources as source (source.id)}
        <label class="choice">
          <input type="checkbox" value={source.id} bind:group={chosen} />
          {source.title}{source.part ? ` — ${source.part}` : ""}
        </label>
      {:else}
        <p class="note">No PDF loaded yet (tab Documents of standards); the catalog can also be written by hand.</p>
      {/each}
    </fieldset>
    <div class="buttons">
      <button type="submit">Make</button>
      <button type="button" onclick={onclose}>Cancel</button>
    </div>
  </form>
</div>
