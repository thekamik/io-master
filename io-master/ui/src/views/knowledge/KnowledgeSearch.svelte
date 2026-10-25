<!--
  The element "search" of the view Knowledge (RAG) (its command
  knowledge_search_run in src/views/knowledge/knowledge.rs): the passages
  of a base that fit the words of the evaluator, the best first, with their
  source, page or requirement, and relevance (the BM25 score of SQLite,
  turned positive: the higher, the better the passage fits). It shows what an agent would get
  for the same words.
-->
<script lang="ts">
  import { sentence } from "../../lib/api/errors";
  import * as knowledge from "../../lib/api/knowledge";

  let { base }: { base: number } = $props();

  let text = $state("");
  let hits: knowledge.HitView[] | null = $state(null);
  let problem = $state("");

  async function search(event: SubmitEvent) {
    event.preventDefault();
    problem = "";

    try {
      hits = await knowledge.knowledgeSearchRun(base, text);
    } catch (error) {
      problem = sentence(error);
    }
  }

  /** Where a passage stands: its requirement and its page, if it has them. */
  function place(hit: knowledge.HitView): string {
    const parts = [hit.source];

    if (hit.label) parts.push(hit.label);
    if (hit.page !== null) parts.push(`page ${hit.page}`);

    return parts.join(" · ");
  }
</script>

<div class="card knowledge-search">
  <h3>Search the base</h3>
  <p class="note">The words are looked for in the passages, also in other forms ("keys" finds "key"); the passages that hold the rarest of them come first. An agent gets its passages the same way.</p>
  <form class="buttons" onsubmit={search}>
    <input bind:value={text} placeholder="How are cryptographic keys generated? FCS_CKM.1" />
    <button type="submit">Search</button>
  </form>

  {#if problem}<p class="problem">{problem}</p>{/if}

  {#if hits !== null && hits.length === 0}
    <p class="note">No passage holds these words.</p>
  {:else if hits}
    {#each hits as hit (hit.id)}
      <div class="link-row">
        <p class="note">{place(hit)} · relevance {(-hit.score).toFixed(1)}</p>
        <blockquote>{hit.text}</blockquote>
      </div>
    {/each}
  {/if}
</div>
