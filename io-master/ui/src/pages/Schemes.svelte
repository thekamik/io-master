<!--
  The tool Create evaluation scheme (steps 16 to 27 of
  docs/todo.md), in two tabs:

  - Test projects: the maps of standards, made here or imported from a
    file (a head of the laboratory gives them to the evaluators); a
    project holds one scheme per chapter of the standard and is opened
    in ProjectEditor.svelte;
  - Documents: the documents of standards (PDF), loaded into the
    encrypted database, with their text page by page, searched and
    shown in the viewer.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import {
    createProject,
    deleteDocument,
    deleteProject,
    importProject,
    listDocuments,
    listProjects,
    loadDocuments,
    searchDocuments,
    sentence,
    stopWork,
    updateDocument,
    type DocumentView,
    type HitView,
    type LoadEvent,
    type ProjectView,
  } from "../lib/api";
  import PdfViewer from "./PdfViewer.svelte";
  import ProjectEditor from "./ProjectEditor.svelte";

  type Tab = "projects" | "documents";

  let tab: Tab = $state("projects");
  let projects: ProjectView[] = $state([]);
  let documents: DocumentView[] = $state([]);
  let problem = $state("");
  let notice = $state("");

  /** The project open. */
  let open = $state<number | null>(null);

  let creating = $state<{ name: string; author: string; documents: number[] } | null>(null);
  let confirmDelete = $state<number | null>(null);

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
      [projects, documents] = await Promise.all([listProjects(), listDocuments()]);
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

  async function importFile() {
    loading = true;
    problem = "";
    notice = "";
    log = [];

    try {
      const imported = await importProject(loadEvent);

      if (imported) {
        await refresh();
        notice = `The test project ${imported.project.name} was imported.`;
        open = imported.project.id;
      }
    } catch (error) {
      problem = `The test project was not imported: ${sentence(error)}`;
    } finally {
      loading = false;
      progress = "";
    }
  }

  async function create(event: SubmitEvent) {
    event.preventDefault();

    if (!creating) {
      return;
    }

    try {
      const made = await createProject({ name: creating.name, version: "1.0", description: "", author: creating.author, documents: creating.documents });
      creating = null;
      await refresh();
      open = made.project.id;
    } catch (error) {
      problem = `The test project was not made: ${sentence(error)}`;
    }
  }

  async function removeProject(id: number) {
    try {
      await deleteProject(id);
      confirmDelete = null;
      await refresh();
    } catch (error) {
      problem = sentence(error);
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

{#if open !== null}
  <ProjectEditor
    projectId={open}
    {documents}
    onclose={() => {
      open = null;
      refresh();
    }}
  />
{:else}
  <div class="tabs">
    <button type="button" class:active={tab === "projects"} onclick={() => (tab = "projects")}>Test projects</button>
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

  {#if tab === "projects"}
    <div class="buttons section-actions">
      <button type="button" class="primary" onclick={() => (creating = { name: "", author: "", documents: documents.map((document) => document.id) })}>
        New test project
      </button>
      <button type="button" onclick={importFile} disabled={loading}>Import a test project…</button>
    </div>

    {#if projects.length === 0}
      <div class="card">
        <h3>No test project yet</h3>
        <p class="note">
          A test project maps a standard for evaluations: one scheme per chapter, each with its inputs, requirements and
          outputs, as tiles joined by links. Load the documents of the standard first (tab Documents of standards), then
          make a project: the AI reads the table of contents and proposes the chapters, then proposes the tiles of each;
          you correct and accept. A project made by the head of the laboratory is imported.
        </p>
      </div>
    {:else}
      <div class="scheme-list">
        {#each projects as project (project.id)}
          <div class="card scheme-card">
            <div class="scheme-card-head">
              <h3>{project.name}</h3>
              <span class="badge {project.ready ? 'success' : ''}">{project.ready ? "ready" : "draft"}</span>
            </div>
            <p class="note">
              {project.version ? `Version ${project.version}` : "No version"}{project.author ? ` · ${project.author}` : ""} ·
              {project.documents.length} document{project.documents.length === 1 ? "" : "s"}
            </p>
            {#if project.description}<p>{project.description}</p>{/if}
            <div class="buttons">
              <button type="button" class="primary" onclick={() => (open = project.id)}>Open</button>
              {#if confirmDelete === project.id}
                <span class="note">Delete the project with its schemes?</span>
                <button type="button" class="danger" onclick={() => removeProject(project.id)}>Delete</button>
                <button type="button" onclick={() => (confirmDelete = null)}>Keep</button>
              {:else}
                <button type="button" class="danger" onclick={() => (confirmDelete = project.id)}>Delete</button>
              {/if}
            </div>
          </div>
        {/each}
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
{/if}

{#if creating}
  <div class="overlay" role="presentation">
    <form class="dialog" onsubmit={create}>
      <h2>New test project</h2>
      <label>Name <input bind:value={creating.name} required placeholder="EN 18031-1" /></label>
      <label>Author <input bind:value={creating.author} placeholder="Head of the laboratory" /></label>
      <fieldset class="flat">
        <legend>Documents of the standard</legend>
        {#each documents as document (document.id)}
          <label class="choice">
            <input type="checkbox" bind:group={creating.documents} value={document.id} />
            {title(document)}
          </label>
        {:else}
          <p class="note">No document is loaded yet; they can be added later in the settings of the project.</p>
        {/each}
      </fieldset>
      <div class="buttons">
        <button type="submit">Make the project</button>
        <button type="button" onclick={() => (creating = null)}>Cancel</button>
      </div>
    </form>
  </div>
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
