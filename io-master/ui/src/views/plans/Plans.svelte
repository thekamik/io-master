<!--
  The view Evaluation plan (src/views/plans/), in two tabs:

  - OSCAL: the documents of the standards in NIST OSCAL (catalogs,
    mappings, profiles, assessment plans), made or imported; a catalog
    opens in its editor (components/CatalogEditor.svelte);
  - Documents of standards: the PDFs, loaded into the encrypted database,
    searched and shown in the viewer; the sources the OSCAL documents
    quote.

  The view keeps only what is presentation: the tab, the open catalog,
  the dialog of a new catalog, and what the viewer shows.
-->
<script lang="ts">
  import CatalogEditor from "../../components/CatalogEditor.svelte";
  import DocumentSearch from "./DocumentSearch.svelte";
  import NewCatalog from "./NewCatalog.svelte";
  import OscalDocuments from "./OscalDocuments.svelte";
  import PdfViewer from "./PdfViewer.svelte";
  import StandardDocuments from "./StandardDocuments.svelte";

  type Tab = "oscal" | "documents";

  let tab: Tab = $state("oscal");
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
  <button type="button" class:active={tab === "oscal"} onclick={() => (tab = "oscal")}>OSCAL</button>
  <button type="button" class:active={tab === "documents"} onclick={() => (tab = "documents")}>
    Documents of standards
  </button>
</div>

{#if tab === "oscal" && catalog}
  <CatalogEditor uuid={catalog} onclose={() => (catalog = null)} />
{:else if tab === "oscal"}
  <OscalDocuments onopen={(uuid) => (catalog = uuid)} onnew={() => (making = true)} />
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
