<!--
  The view Converter (src/views/converter/): the documents of a standard
  (PDF) converted into a catalog of NIST OSCAL with the help of the AI, its
  only aim (step 50 of docs/history.md). In two tabs:

  - Catalogs: the catalogs of OSCAL; a new one is made from the PDFs, and
    each opens in its editor (catalog_editor/), where the AI proposes its
    groups and controls and the evaluator accepts them;
  - Documents of standards: the PDFs, loaded into the encrypted database,
    searched and shown in the viewer.

  The view keeps only what is presentation: the tab, the open catalog, the
  dialog of a new catalog, and what the viewer shows. The finished documents
  of OSCAL are used in the view Evaluation plan.
-->
<script lang="ts">
  import CatalogEditor from "./catalog_editor/CatalogEditor.svelte";
  import CatalogList from "./CatalogList.svelte";
  import DocumentSearch from "./DocumentSearch.svelte";
  import NewCatalog from "./NewCatalog.svelte";
  import PdfViewer from "../../components/pdf_viewer/PdfViewer.svelte";
  import StandardDocuments from "./StandardDocuments.svelte";

  type Tab = "catalogs" | "documents";

  let tab: Tab = $state("catalogs");
  /** The catalog open in its editor. */
  let catalog: string | null = $state(null);
  let making = $state(false);
  let viewer: { documentId: number; page: number; quote: string } | null = $state(null);

  function made(uuid: string) {
    making = false;
    catalog = uuid;
  }
</script>

<div class="tabs">
  <button type="button" class:active={tab === "catalogs"} onclick={() => (tab = "catalogs")}>Catalogs</button>
  <button type="button" class:active={tab === "documents"} onclick={() => (tab = "documents")}>
    Documents of standards
  </button>
</div>

{#if tab === "catalogs" && catalog}
  <CatalogEditor uuid={catalog} onclose={() => (catalog = null)} />
{:else if tab === "catalogs"}
  <CatalogList onopen={(uuid) => (catalog = uuid)} onnew={() => (making = true)} />
{:else}
  <DocumentSearch onopen={(documentId, page, quote) => (viewer = { documentId, page, quote })} />
  <StandardDocuments onview={(documentId) => (viewer = { documentId, page: 1, quote: "" })} />
{/if}

{#if making}
  <NewCatalog onmade={made} onclose={() => (making = false)} />
{/if}

{#if viewer}
  <PdfViewer documentId={viewer.documentId} page={viewer.page} quote={viewer.quote} onclose={() => (viewer = null)} />
{/if}
