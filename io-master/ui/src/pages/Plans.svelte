<!--
  The evaluation plan (step 41 of docs/todo.md, docs/oscal.md), in two
  tabs:

  - OSCAL: the documents of the standards in NIST OSCAL (catalogs,
    mappings, profiles, assessment plans), made or loaded, and the plan
    shown as a graph (built in steps 41.2 to 41.4);
  - Documents: the documents of standards (PDF), loaded into the
    encrypted database, with their text page by page, searched and
    shown in the viewer; the sources the OSCAL documents quote.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { deleteDocument, listDocuments, loadDocuments, searchDocuments, stopWork, updateDocument, type DocumentView, type HitView, type LoadEvent } from "../lib/api/documents";
import { sentence } from "../lib/api/errors";
  import PdfViewer from "../components/PdfViewer.svelte";

  type Tab = "oscal" | "documents";

  let tab: Tab = $state("documents");
  let documents: DocumentView[] = $state([]);
  let problem = $state("");
  let notice = $state("");


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
      documents = await listDocuments();
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

  {#if tab === "oscal"}
    <div class="card">
      <h3>The standards in NIST OSCAL</h3>
      <p class="note">
        The evaluation plan is built on NIST OSCAL: the requirements of a standard as a catalog, the choice for one
        evaluation as a profile, the plan as an assessment plan, shown as a graph. This tab comes with step 41.2;
        until then, load the documents of the standards (tab Documents of standards).
      </p>
    </div>
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

{#if viewer}
  <PdfViewer {documents} documentId={viewer.documentId} page={viewer.page} quote={viewer.quote} onclose={() => (viewer = null)} />
{/if}
