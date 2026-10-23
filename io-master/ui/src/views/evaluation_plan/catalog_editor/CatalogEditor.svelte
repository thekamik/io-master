<!--
  The view "editor of a catalog" (src/views/evaluation_plan/catalog_editor/): a catalog of
  OSCAL made in the program from the documents (PDF) of its standard.

  - On the left the tree of the catalog.
  - In the middle the object chosen: its parts, its quotes checked on their
    pages, its own fields as JSON of OSCAL.
  - On the right the AI, and the proposals waiting for the evaluator.
    Nothing the AI proposes enters the catalog until the evaluator accepts
    it, as it is or corrected (P1).

  The view keeps only what is presentation: the object chosen, the dialog
  of a new object, the instructions open, what the viewer shows. When an
  element changed the catalog, the view counts up `version`, and every
  element reads its data again.
-->
<script lang="ts">
  import type { Which } from "../../../lib/api/evaluation_plan/catalog_editor/ai_instructions";
  import type { CatalogTreeView } from "../../../lib/api/evaluation_plan/catalog_editor/catalog_tree";
  import type { Place } from "../../../lib/api/evaluation_plan/catalog_editor/new_object";
  import PdfViewer from "../../../components/pdf_viewer/PdfViewer.svelte";
  import AiBuilder from "./AiBuilder.svelte";
  import AiInstructions from "./AiInstructions.svelte";
  import CatalogTree from "./CatalogTree.svelte";
  import NewObject from "./NewObject.svelte";
  import ObjectForm from "./ObjectForm.svelte";
  import ObjectQuotes from "./ObjectQuotes.svelte";
  import Proposals from "./Proposals.svelte";

  let { uuid, onclose }: { uuid: string; onclose: () => void } = $props();

  /** Counted up after every change: the elements read their data again. */
  let version = $state(0);
  let head: CatalogTreeView | null = $state(null);
  let selected: { id: string; kind: "group" | "control" } | null = $state(null);
  let adding: { kind: "group" | "control"; place: Place } | null = $state(null);
  let instructions: Which | null = $state(null);
  /** The viewer: a quote shown, or (quoting) a passage to take for the object chosen. */
  let viewer: { documentId: number; page: number; quote: string; quoting: boolean } | null = $state(null);
  let quotes: ObjectQuotes | null = $state(null);

  function changed() {
    version += 1;
  }

  /** After a change of the object chosen: its id now (null: deleted). */
  function objectChanged(id: string | null) {
    selected = id && selected ? { id, kind: selected.kind } : null;
    changed();
  }

  function added(id: string | null) {
    if (id && adding) {
      selected = { id, kind: adding.kind };
    }

    adding = null;
    changed();
  }

  function taken(source: { document_id: number; page: number; quote: string }) {
    viewer = null;
    quotes?.add(source);
  }
</script>

<div class="editor-head">
  <button type="button" class="small" onclick={onclose}>← All OSCAL documents</button>
  {#if head}
    <h2>{head.title}</h2>
    <span class="note">{head.version} · last modified {head.last_modified}</span>
  {/if}
</div>

<div class="catalog-body">
  <CatalogTree
    {uuid}
    {version}
    selected={selected?.id ?? null}
    onchoose={(id, kind) => (selected = { id, kind })}
    onread={(tree) => (head = tree)}
    onadd={(kind) => (adding = { kind, place: { kind: "top" } })}
  />

  {#if selected}
    <ObjectForm
      {uuid}
      id={selected.id}
      {version}
      onchanged={objectChanged}
      onadd={(kind, place) => (adding = { kind, place })}
    >
      <ObjectQuotes
        bind:this={quotes}
        {uuid}
        objectId={selected.id}
        {version}
        onview={(documentId, page, quote) => (viewer = { documentId, page, quote, quoting: false })}
        onquote={(documentId, page) => (viewer = { documentId, page, quote: "", quoting: true })}
        onchanged={changed}
      />
    </ObjectForm>
  {:else}
    <div class="card catalog-object"><p class="note">Choose a group or a control in the tree.</p></div>
  {/if}

  <div class="card catalog-ai">
    <AiBuilder
      {uuid}
      group={selected?.kind === "group" ? selected.id : null}
      onchanged={changed}
      oninstructions={(which) => (instructions = which)}
    />
    <Proposals
      {uuid}
      {version}
      onview={(documentId, page, quote) => (viewer = { documentId, page, quote, quoting: false })}
      onchanged={changed}
    />
  </div>
</div>

{#if adding}
  <NewObject {uuid} kind={adding.kind} place={adding.place} ondone={added} onclose={() => (adding = null)} />
{/if}

{#if instructions}
  <AiInstructions which={instructions} onclose={() => (instructions = null)} />
{/if}

{#if viewer}
  <PdfViewer
    documentId={viewer.documentId}
    page={viewer.page}
    quote={viewer.quote}
    onquote={viewer.quoting ? taken : undefined}
    onclose={() => (viewer = null)}
  />
{/if}
