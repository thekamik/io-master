<!--
  The element "search" of the view Converter
  (its commands in src/views/converter/converter.rs): the text of the documents of
  standards searched. Opening a hit only tells the view (onopen), which
  shows it in the viewer.
-->
<script lang="ts">
  import { sentence } from "../../lib/api/errors";
  import * as converter from "../../lib/api/converter";

  let { onopen }: { onopen: (documentId: number, page: number, quote: string) => void } = $props();

  let query = $state("");
  let searched = $state("");
  let hits: converter.HitView[] = $state([]);
  let problem = $state("");

  async function search(event: SubmitEvent) {
    event.preventDefault();
    problem = "";

    try {
      hits = await converter.documentSearchRun(query, null);
      searched = query;
    } catch (error) {
      problem = sentence(error);
    }
  }
</script>

<div class="buttons section-actions">
  <form class="search" onsubmit={search}>
    <input bind:value={query} placeholder="Search the text of the documents" />
    <button type="submit" disabled={!query.trim()}>Search</button>
  </form>
</div>

{#if problem}<p class="problem">{problem}</p>{/if}

{#if hits.length > 0}
  <div class="card">
    <h3>
      {hits.length} hit{hits.length === 1 ? "" : "s"}
      <button type="button" class="small" onclick={() => (hits = [])}>Clear</button>
    </h3>
    <ul class="hits">
      {#each hits as hit, index (index)}
        <li>
          <button type="button" class="link" onclick={() => onopen(hit.document_id, hit.page, searched)}>
            <span class="note">{hit.document || "?"}, page {hit.page}</span>
            <span>{hit.snippet}</span>
          </button>
        </li>
      {/each}
    </ul>
  </div>
{/if}
