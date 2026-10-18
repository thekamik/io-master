<!--
  The evaluation plan (step 41 of docs/todo.md, docs/oscal.md), in two
  tabs:

  - OSCAL: the documents of the standards in NIST OSCAL (catalogs,
    mappings, profiles, assessment plans), made or loaded; a catalog
    made in the program is edited in components/CatalogEditor.svelte
    (step 41.3), the plan is shown as a graph (step 41.4);
  - Documents: the documents of standards (PDF), loaded into the
    encrypted database, with their text page by page, searched and
    shown in the viewer; the sources the OSCAL documents quote.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { deleteDocument, listDocuments, loadDocuments, searchDocuments, stopWork, updateDocument, type DocumentView, type HitView, type LoadEvent } from "../lib/api/documents";
  import { createCatalog } from "../lib/api/catalogs";
  import { sentence } from "../lib/api/errors";
  import { deleteOscalDocument, exportOscalDocument, importOscalDocument, listOscalDocuments, type OscalDocumentView } from "../lib/api/oscal";
  import CatalogEditor from "../components/CatalogEditor.svelte";
  import PdfViewer from "../components/PdfViewer.svelte";

  type Tab = "oscal" | "documents";

  let tab: Tab = $state("oscal");
  let oscal: OscalDocumentView[] = $state([]);
  let confirmDelete = $state<string | null>(null);
  let documents: DocumentView[] = $state([]);
  let problem = $state("");
  let notice = $state("");

  // The catalog open in the editor, and a new catalog being made.
  let catalog = $state<string | null>(null);
  let making = $state<{ title: string; version: string; sources: number[] } | null>(null);

  // Loading documents and importing.
  let loading = $state(false);
  let progress = $state("");
  let log: { done: boolean; text: string }[] = $state([]);

  // Documents.
  let editing = $state<DocumentView | null>(null);
  let query = $state("");
  let hits: HitView[] = $state([]);
  let viewer = $state<{ documentId: number; page: number; quote: string } | null>(null);

  async function refresh() {
    try {
      [documents, oscal] = await Promise.all([listDocuments(), listOscalDocuments()]);
    } catch (error) {
      problem = sentence(error);
    }
  }

  onMount(refresh);

  function title(document: DocumentView): string {
    return `${document.title}${document.part ? ` — ${document.part}` : ""}`;
  }

  function loadEvent(event: LoadEvent) {
    switch (event.event) {
      case "file":
        progress = `Reading ${event.name} (${event.index} of ${event.count})…`;
        break;
      case "pages":
        progress = progress.replace(/ — page .*$/, "") + ` — page ${event.done} of ${event.total}`;
        break;
      case "loaded":
        log.push({ done: true, text: `Loaded: ${title(event.document)}, ${event.document.page_count} pages.` });
        break;
      case "failed":
        log.push({ done: false, text: `${event.name}: ${event.message}` });
        break;
      case "end":
        progress = event.stopped ? "Stopped." : "";
        break;
    }
  }

  async function load() {
    loading = true;
    problem = "";
    log = [];

    try {
      await loadDocuments(loadEvent);
      await refresh();
    } catch (error) {
      problem = sentence(error);
    } finally {
      loading = false;
    }
  }


  async function importOscal() {
    problem = "";
    notice = "";

    try {
      const imported = await importOscalDocument();

      if (imported) {
        notice = `Imported: ${imported.model_title} "${imported.title}".`;
        await refresh();
      }
    } catch (error) {
      problem = `The OSCAL document was not imported: ${sentence(error)}`;
    }
  }

  async function exportOscal(document: OscalDocumentView) {
    problem = "";
    notice = "";

    try {
      if (await exportOscalDocument(document.uuid, document.title)) {
        notice = `Exported: ${document.title}.`;
      }
    } catch (error) {
      problem = `The OSCAL document was not exported: ${sentence(error)}`;
    }
  }

  async function removeOscal(uuid: string) {
    try {
      await deleteOscalDocument(uuid);
      confirmDelete = null;
      await refresh();
    } catch (error) {
      problem = sentence(error);
    }
  }

  async function makeCatalog(event: SubmitEvent) {
    event.preventDefault();

    if (making) {
      try {
        const made = await createCatalog(making.title, making.version, making.sources);
        making = null;
        await refresh();
        catalog = made.uuid;
      } catch (error) {
        problem = `The catalog was not made: ${sentence(error)}`;
      }
    }
  }

  async function saveDocument(event: SubmitEvent) {
    event.preventDefault();

    if (editing) {
      try {
        await updateDocument(editing.id, { title: editing.title, part: editing.part, version: editing.version });
        editing = null;
        await refresh();
      } catch (error) {
        problem = `The document was not saved: ${sentence(error)}`;
      }
    }
  }

  async function removeDocument(id: number) {
    try {
      await deleteDocument(id);
      await refresh();
    } catch (error) {
      problem = `The document was not deleted: ${sentence(error)}`;
    }
  }

  async function search(event: SubmitEvent) {
    event.preventDefault();

    try {
      hits = await searchDocuments(query, null);
    } catch (error) {
      problem = sentence(error);
    }
  }
</script>

<div class="tabs">
    <button type="button" class:active={tab === "oscal"} onclick={() => (tab = "oscal")}>OSCAL</button>
    <button type="button" class:active={tab === "documents"} onclick={() => (tab = "documents")}>
      Documents of standards
    </button>
  </div>

  {#if problem}<p class="problem">{problem}</p>{/if}
  {#if notice}<p class="done">{notice}</p>{/if}

  {#if loading || progress || log.length > 0}
    <div class="card load-log">
      {#if progress}<p class="note">{progress}</p>{/if}
      {#each log as line, index (index)}
        <div class={line.done ? "done" : "problem"}>{line.text}</div>
      {/each}
      {#if loading}
        <button type="button" class="small danger" onclick={stopWork}>Stop</button>
      {:else}
        <button type="button" class="small" onclick={() => (log = [])}>Close</button>
      {/if}
    </div>
  {/if}

  {#if tab === "oscal" && catalog}
    <CatalogEditor
      uuid={catalog}
      {documents}
      onclose={() => {
        catalog = null;
        refresh();
      }}
    />
  {:else if tab === "oscal"}
    <div class="buttons section-actions">
      <button type="button" class="primary" onclick={() => (making = { title: "", version: "", sources: [] })}>New catalog…</button>
      <button type="button" onclick={importOscal}>Import an OSCAL document…</button>
    </div>

    {#if oscal.length === 0}
      <div class="card">
        <h3>No OSCAL document yet</h3>
        <p class="note">
          The evaluation plan is built on NIST OSCAL: the requirements of a standard as a catalog, the relations
          between documents as a mapping, the choice for one evaluation as a profile, the plan as an assessment plan.
          Make a catalog of a standard from its documents (PDF), with the help of the AI, or import a document of OSCAL
          (JSON, version 1.x). Every document is checked against the schema of NIST.
        </p>
      </div>
    {:else}
      <div class="table-card">
        <table class="ports documents">
          <thead>
            <tr><th>Model</th><th>Title</th><th>Version</th><th>OSCAL</th><th>Last modified</th><th></th></tr>
          </thead>
          <tbody>
            {#each oscal as document (document.uuid)}
              <tr>
                <td>{document.model_title}</td>
                <td>{document.title}</td>
                <td>{document.version}</td>
                <td class="note">{document.oscal_version}</td>
                <td class="note">{document.last_modified}</td>
                <td class="actions">
                  {#if document.model === "catalog"}
                    <button type="button" class="small" onclick={() => (catalog = document.uuid)}>Open</button>
                  {/if}
                  <button type="button" class="small" onclick={() => exportOscal(document)}>Export</button>
                  {#if confirmDelete === document.uuid}
                    <button type="button" class="small danger" onclick={() => removeOscal(document.uuid)}>Delete</button>
                    <button type="button" class="small" onclick={() => (confirmDelete = null)}>Keep</button>
                  {:else}
                    <button type="button" class="small danger" onclick={() => (confirmDelete = document.uuid)}>Delete</button>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  {:else}
    <div class="buttons section-actions">
      <button type="button" class="primary" onclick={load} disabled={loading}>Load PDFs…</button>
      <form class="search" onsubmit={search}>
        <input bind:value={query} placeholder="Search the text of the documents" />
        <button type="submit" disabled={!query.trim()}>Search</button>
      </form>
    </div>

    {#if hits.length > 0}
      <div class="card">
        <h3>{hits.length} hit{hits.length === 1 ? "" : "s"} <button type="button" class="small" onclick={() => (hits = [])}>Clear</button></h3>
        <ul class="hits">
          {#each hits as hit, index (index)}
            {@const document = documents.find((candidate) => candidate.id === hit.document_id)}
            <li>
              <button type="button" class="link" onclick={() => (viewer = { documentId: hit.document_id, page: hit.page, quote: query })}>
                <span class="note">{document ? title(document) : "?"}, page {hit.page}</span>
                <span>{hit.snippet}</span>
              </button>
            </li>
          {/each}
        </ul>
      </div>
    {/if}

    {#if documents.length === 0}
      <div class="card">
        <h3>No document yet</h3>
        <p class="note">
          Load the documents of a standard as PDFs (a standard often comes in several parts: load each of them). Their
          text is read page by page and kept, with the files, in your encrypted database.
        </p>
      </div>
    {:else}
      <div class="table-card">
        <table class="ports documents">
          <thead>
            <tr><th>Title</th><th>Part</th><th>Version</th><th>Pages</th><th>File</th><th></th></tr>
          </thead>
          <tbody>
            {#each documents as document (document.id)}
              <tr>
                <td>{document.title}</td>
                <td>{document.part}</td>
                <td>{document.version}</td>
                <td>{document.page_count}</td>
                <td class="note">{document.file_name}</td>
                <td class="actions">
                  <button type="button" class="small" onclick={() => (viewer = { documentId: document.id, page: 1, quote: "" })}>View</button>
                  <button type="button" class="small" onclick={() => (editing = { ...document })}>Edit</button>
                  <button type="button" class="small danger" onclick={() => removeDocument(document.id)}>Delete</button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  {/if}

{#if editing}
  <div class="overlay" role="presentation">
    <form class="dialog" onsubmit={saveDocument}>
      <h2>The document</h2>
      <label>Title <input bind:value={editing.title} required /></label>
      <div class="row">
        <label>Part <input bind:value={editing.part} placeholder="Part 3: Security assurance components" /></label>
        <label>Version <input bind:value={editing.version} placeholder="CC:2022 Revision 1" /></label>
      </div>
      <p class="note mono">{editing.file_name} · SHA-256 {editing.sha256.slice(0, 16)}…</p>
      <div class="buttons">
        <button type="submit">Save</button>
        <button type="button" onclick={() => (editing = null)}>Cancel</button>
      </div>
    </form>
  </div>
{/if}

{#if making}
  <div class="overlay" role="presentation">
    <form class="dialog" onsubmit={makeCatalog}>
      <h2>A new catalog</h2>
      <p class="note">
        A standard, its methodology (the CEM), a Protection Profile or a test specification: each is a catalog of its own.
      </p>
      <label>Title <input bind:value={making.title} required placeholder="Common Criteria Part 2: Security functional components" /></label>
      <label>Version <input bind:value={making.version} placeholder="CC:2022 Revision 1" /></label>
      <fieldset>
        <legend>Comes from the documents</legend>
        {#each documents as document (document.id)}
          <label class="choice"><input type="checkbox" value={document.id} bind:group={making.sources} /> {title(document)}</label>
        {:else}
          <p class="note">No PDF loaded yet (tab Documents of standards); the catalog can also be written by hand.</p>
        {/each}
      </fieldset>
      <div class="buttons">
        <button type="submit">Make</button>
        <button type="button" onclick={() => (making = null)}>Cancel</button>
      </div>
    </form>
  </div>
{/if}

{#if viewer}
  <PdfViewer {documents} documentId={viewer.documentId} page={viewer.page} quote={viewer.quote} onclose={() => (viewer = null)} />
{/if}
