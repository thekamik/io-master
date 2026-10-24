<!--
  The element "proposals" of the editor of a catalog
  (its commands in src/views/converter/catalog_editor/catalog_editor.rs): what the AI proposed, waiting for
  the evaluator, who accepts it (as it is, or corrected as JSON of OSCAL)
  or rejects it (P1: the evaluator decides). A change tells the view
  (onchanged); showing a quote only tells the view (onview).
-->
<script lang="ts">
  import { sentence } from "../../../lib/api/errors";
  import * as catalogEditor from "../../../lib/api/converter/catalog_editor";
  import PartRows from "../../../components/part_rows/PartRows.svelte";

  let {
    uuid,
    version,
    onview,
    onchanged,
  }: {
    uuid: string;
    version: number;
    onview: (documentId: number, page: number, quote: string) => void;
    onchanged: () => void;
  } = $props();

  let waiting: catalogEditor.ProposalView[] = $state([]);
  let correcting: { id: number; text: string } | null = $state(null);
  let problem = $state("");

  $effect(() => {
    void version;
    load(uuid);
  });

  async function load(catalog: string) {
    try {
      waiting = await catalogEditor.proposalsGet(catalog);
      problem = "";
    } catch (error) {
      problem = sentence(error);
    }
  }

  async function accept(id: number, corrected: unknown | null) {
    try {
      await catalogEditor.proposalsAccept(id, corrected);
      correcting = null;
      onchanged();
    } catch (error) {
      problem = sentence(error);
    }
  }

  function acceptCorrected() {
    if (!correcting) {
      return;
    }

    try {
      accept(correcting.id, JSON.parse(correcting.text));
    } catch (error) {
      problem = `The text is not JSON: ${error instanceof Error ? error.message : String(error)}`;
    }
  }

  async function reject(id: number) {
    try {
      await catalogEditor.proposalsReject(id);
      onchanged();
    } catch (error) {
      problem = sentence(error);
    }
  }
</script>

<h4>Proposals ({waiting.length})</h4>
{#if problem}<p class="problem">{problem}</p>{/if}
{#each waiting as proposal (proposal.id)}
  <div class="proposal">
    <div class="note">
      {proposal.kind === "group" ? "Group" : "Control"} in {proposal.parent_id ?? "the top"} · {proposal.model}
    </div>
    <strong><span class="object-id">{proposal.object_id}</span> {proposal.title}</strong>
    <PartRows rows={proposal.parts} />
    {#each proposal.quotes as quote, index (index)}
      <div class="source">
        <button type="button" class="link" onclick={() => onview(quote.source_document_id, quote.page, quote.text)}>
          {quote.document}, page {quote.page}
        </button>
        {#if !quote.found}<span class="tile-flag warn">not found on its page</span>{/if}
        <blockquote>{quote.text}</blockquote>
      </div>
    {/each}
    {#if proposal.note}<p class="note">AI note: {proposal.note}</p>{/if}

    {#if correcting?.id === proposal.id}
      <textarea class="mono" rows="10" bind:value={correcting.text}></textarea>
      <div class="buttons">
        <button type="button" class="small" onclick={acceptCorrected}>Accept corrected</button>
        <button type="button" class="small" onclick={() => (correcting = null)}>Cancel</button>
      </div>
    {:else}
      <div class="buttons">
        <button type="button" class="small" onclick={() => accept(proposal.id, null)}>Accept</button>
        <button
          type="button"
          class="small"
          onclick={() => (correcting = { id: proposal.id, text: JSON.stringify(proposal.content, null, 2) })}>Correct…</button
        >
        <button type="button" class="small danger" onclick={() => reject(proposal.id)}>Reject</button>
      </div>
    {/if}
  </div>
{:else}
  <p class="note">No proposal waits for you.</p>
{/each}
