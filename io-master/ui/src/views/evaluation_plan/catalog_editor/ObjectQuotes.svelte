<!--
  The element "quotes" of the editor of a catalog
  (src/views/evaluation_plan/catalog_editor/object_quotes/): the passages of the standard
  the object comes from, each looked for on its page. Showing a quote and
  "Add a quote…" only tell the view, which opens the viewer; the view
  gives back the passage selected there (add(), below).
-->
<script lang="ts">
  import { sentence } from "../../../lib/api/errors";
  import * as objectQuotes from "../../../lib/api/evaluation_plan/catalog_editor/object_quotes";

  let {
    uuid,
    objectId,
    version,
    onview,
    onquote,
    onchanged,
  }: {
    uuid: string;
    objectId: string;
    version: number;
    onview: (documentId: number, page: number, quote: string) => void;
    onquote: (documentId: number, page: number) => void;
    onchanged: () => void;
  } = $props();

  let quotes: objectQuotes.ObjectQuotesView | null = $state(null);
  let problem = $state("");

  $effect(() => {
    void version;
    load(uuid, objectId);
  });

  async function load(catalog: string, object: string) {
    try {
      quotes = await objectQuotes.get(catalog, object);
      problem = "";
    } catch (error) {
      problem = sentence(error);
    }
  }

  /** Adds the passage selected in the viewer; called by the view. */
  export async function add(source: { document_id: number; page: number; quote: string }) {
    try {
      await objectQuotes.add(uuid, objectId, source.document_id, source.page, source.quote);
      onchanged();
    } catch (error) {
      problem = `The quote was not added: ${sentence(error)}`;
    }
  }

  async function remove(id: number) {
    try {
      await objectQuotes.remove(id);
      onchanged();
    } catch (error) {
      problem = sentence(error);
    }
  }

  function quote() {
    const first = quotes?.quotes[0];

    if (first) {
      onquote(first.source_document_id, first.page);
    } else if (quotes?.first_source) {
      onquote(quotes.first_source, 1);
    }
  }
</script>

<h4>Quotes</h4>
{#if problem}<p class="problem">{problem}</p>{/if}
{#if quotes}
  {#each quotes.quotes as item (item.id)}
    <div class="source">
      <div class="source-head">
        <button type="button" class="link" onclick={() => onview(item.source_document_id, item.page, item.text)}>
          {item.document}, page {item.page}
        </button>
        {#if !item.found}<span class="tile-flag warn">not found on its page</span>{/if}
        <button type="button" class="small danger" onclick={() => remove(item.id)}>Delete</button>
      </div>
      <blockquote>{item.text}</blockquote>
    </div>
  {:else}
    <p class="note">No quote yet: the passage of the standard this object comes from.</p>
  {/each}
  <button type="button" class="small" disabled={quotes.first_source === null} onclick={quote}>
    Add a quote from the standard…
  </button>
{/if}
