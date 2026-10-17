<!--
  An open test project (steps 25 to 27 of docs/todo.md): on the left
  the list of its schemes, one per chapter of the standard; the scheme
  chosen is drawn on the right (SchemeEditor.svelte).

  - "Read the table of contents with AI" (ContentsPanel.svelte): the
    AI proposes the chapters; they appear in the list as proposed
    schemes, with the pages where they were found, for the user to
    correct, accept or delete.
  - "Analyse chapters with AI" (ChaptersPanel.svelte): the AI reads the
    pages of every chosen scheme and proposes its tiles.
  - The settings of the project (name, version, author, documents of
    the standard), its export as a file, and "ready" are in the head.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { type DocumentView } from "../lib/api/documents";
import { sentence } from "../lib/api/errors";
import { acceptSchemes, createScheme, deleteSchemes, exportProject, openProject, schemeName, setProjectReady, updateProject, updateScheme, type ProjectDataView, type ProjectFields, type SchemeFields, type SchemeSummaryView } from "../lib/api/projects";
  import ChaptersPanel from "./ChaptersPanel.svelte";
  import ContentsPanel from "./ContentsPanel.svelte";
  import SchemeEditor from "./SchemeEditor.svelte";

  let {
    projectId,
    documents,
    onclose,
  }: {
    projectId: number;
    /** Every loaded document, for the settings of the project. */
    documents: DocumentView[];
    onclose: () => void;
  } = $props();

  let data = $state<ProjectDataView | null>(null);
  let problem = $state("");
  let notice = $state("");

  /** The scheme drawn on the right. */
  let open = $state<number | null>(null);

  /** Whether the list of schemes is shown; hidden, the drawing takes the whole width. */
  let listShown = $state(true);

  let settings = $state<ProjectFields | null>(null);
  let exporting = $state(false);
  let withPdfs = $state(false);
  let reading = $state(false);
  let analysing = $state(false);

  /** A scheme being made (id null) or changed. */
  let editing = $state<{ id: number | null; fields: SchemeFields } | null>(null);
  let confirmDelete = $state<number | null>(null);

  let proposed = $derived(data?.schemes.filter((scheme) => scheme.state === "proposed") ?? []);

  async function act(work: () => Promise<ProjectDataView>, done = "") {
    problem = "";
    notice = "";

    try {
      data = await work();
      notice = done;

      if (open !== null && !data.schemes.some((scheme) => scheme.id === open)) {
        open = null;
      }
    } catch (error) {
      problem = sentence(error);
    }
  }

  /** The counts of the list again, after a change in the drawing. */
  async function refresh() {
    try {
      data = await openProject(projectId);
    } catch (error) {
      problem = sentence(error);
    }
  }

  onMount(() => act(() => openProject(projectId)));

  function newScheme() {
    editing = { id: null, fields: { number: "", name: "", pages: null, state: "accepted" } };
  }

  function editScheme(scheme: SchemeSummaryView) {
    editing = {
      id: scheme.id,
      fields: { number: scheme.number, name: scheme.name, pages: scheme.pages ? { ...scheme.pages } : null, state: scheme.state },
    };
  }

  async function saveScheme(event: SubmitEvent) {
    event.preventDefault();

    if (!editing) {
      return;
    }

    const { id, fields } = editing;

    await act(
      () => (id === null ? createScheme(projectId, fields) : updateScheme(id, fields)),
      id === null ? "The scheme was made." : "The scheme was saved.",
    );

    if (!problem) {
      editing = null;

      if (id === null && data) {
        open = data.schemes[data.schemes.length - 1]?.id ?? null;
      }
    }
  }

  function setPages(on: boolean) {
    if (editing) {
      const document = data?.documents[0];
      editing.fields.pages = on && document ? { document_id: document.id, first_page: 1, last_page: document.page_count } : null;
    }
  }

  async function saveSettings(event: SubmitEvent) {
    event.preventDefault();

    if (settings) {
      const fields = settings;
      await act(() => updateProject(projectId, fields), "The project was saved.");

      if (!problem) {
        settings = null;
      }
    }
  }

  async function exportNow() {
    problem = "";

    try {
      if (await exportProject(projectId, withPdfs)) {
        notice = withPdfs ? "The project was exported with the PDFs of its documents." : "The project was exported.";
      }

      exporting = false;
    } catch (error) {
      problem = `The project was not exported: ${sentence(error)}`;
    }
  }

  function toggleReady() {
    if (!data) {
      return;
    }

    const ready = !data.project.ready;
    const findings = data.findings;

    act(
      () => setProjectReady(projectId, ready),
      ready && findings > 0
        ? `The project is marked ready with ${findings} findings accepted.`
        : ready
          ? "The project is marked ready."
          : "The project is a draft again.",
    );
  }
</script>

{#if data}
  <div class="project">
    <header class="editor-head">
      <button type="button" class="ghost" onclick={onclose}>‹ Test projects</button>
      <h2>{data.project.name}</h2>
      {#if data.project.version}<span class="note">version {data.project.version}</span>{/if}
      <span class="badge {data.project.ready ? 'success' : ''}">{data.project.ready ? "ready" : "draft"}</span>

      <div class="buttons push">
        <button type="button" onclick={() => (listShown = !listShown)}>{listShown ? "Hide the list" : "Show the list"}</button>
        <button type="button" onclick={() => (settings = { ...data!.project, documents: [...data!.project.documents] })}>Settings</button>
        <button type="button" onclick={() => (exporting = true)}>Export</button>
        <button type="button" onclick={toggleReady}>{data.project.ready ? "Make a draft" : "Mark ready"}</button>
      </div>
    </header>

    {#if problem}<p class="problem">{problem}</p>{/if}
    {#if notice}<p class="done">{notice}</p>{/if}

    <div class="project-body">
      <aside class="chapters" class:hidden={!listShown}>
        <div class="buttons">
          <button type="button" class="primary small" onclick={() => (reading = true)} disabled={data.documents.length === 0}>
            Read the table of contents with AI
          </button>
          <button type="button" class="small" onclick={() => (analysing = true)} disabled={data.schemes.length === 0}>
            Analyse chapters with AI
          </button>
          <button type="button" class="small" onclick={newScheme}>+ Scheme</button>
        </div>

        {#if proposed.length > 0}
          <div class="buttons">
            <button type="button" class="small" onclick={() => act(() => acceptSchemes(projectId, proposed.map((scheme) => scheme.id)), "All proposed schemes were accepted.")}>
              Accept {proposed.length} proposed
            </button>
            <button type="button" class="small danger" onclick={() => act(() => deleteSchemes(projectId, proposed.map((scheme) => scheme.id)), "All proposed schemes were deleted.")}>
              Delete proposed
            </button>
          </div>
        {/if}

        {#if data.schemes.length === 0}
          <p class="note">
            No scheme yet. A scheme is one chapter of the standard: its requirements, their inputs and outputs. Let the AI
            read the table of contents, or add schemes by hand.
          </p>
        {:else}
          <ul class="chapter-list">
            {#each data.schemes as scheme (scheme.id)}
              <li class:active={scheme.id === open} class:proposed={scheme.state === "proposed"}>
                <button type="button" class="link" onclick={() => (open = scheme.id)}>
                  <span class="chapter-name">{schemeName(scheme)}</span>
                  <span class="note">
                    {scheme.requirements} requirement{scheme.requirements === 1 ? "" : "s"} · {scheme.tiles} tile{scheme.tiles === 1 ? "" : "s"}{scheme.findings > 0 ? ` · ${scheme.findings} findings` : ""}{scheme.proposed > 0 ? ` · ${scheme.proposed} proposed` : ""}
                    {scheme.pages ? ` · p. ${scheme.pages.first_page}–${scheme.pages.last_page}` : " · no pages"}
                  </span>
                </button>
                {#if scheme.id === open}
                  <div class="buttons">
                    {#if scheme.state === "proposed"}
                      <button type="button" class="small" onclick={() => act(() => acceptSchemes(projectId, [scheme.id]), "The scheme was accepted.")}>Accept</button>
                    {/if}
                    <button type="button" class="small" onclick={() => editScheme(scheme)}>Edit</button>
                    {#if confirmDelete === scheme.id}
                      <button type="button" class="small danger" onclick={() => { confirmDelete = null; act(() => deleteSchemes(projectId, [scheme.id]), "The scheme was deleted."); }}>
                        Delete with {scheme.tiles} tiles
                      </button>
                      <button type="button" class="small" onclick={() => (confirmDelete = null)}>Keep</button>
                    {:else}
                      <button type="button" class="small danger" onclick={() => (confirmDelete = scheme.id)}>Delete</button>
                    {/if}
                  </div>
                {/if}
              </li>
            {/each}
          </ul>
        {/if}
      </aside>

      <section class="drawing">
        {#if open !== null}
          {#key open}
            <SchemeEditor schemeId={open} onchange={refresh} onopen={(scheme) => (open = scheme)} />
          {/key}
        {:else}
          <div class="card">
            <h3>Choose a scheme</h3>
            <p class="note">
              {data.schemes.length} schemes, {data.schemes.reduce((sum, scheme) => sum + scheme.requirements, 0)} requirements,
              {data.findings} findings of the checks in the whole project. Choose a scheme on the left to see its graph.
            </p>
          </div>
        {/if}
      </section>
    </div>
  </div>

  {#if editing}
    <div class="overlay" role="presentation">
      <form class="dialog" onsubmit={saveScheme}>
        <h2>{editing.id === null ? "New scheme" : "The scheme"}</h2>
        <div class="row">
          <label>Number <input bind:value={editing.fields.number} placeholder="9.1" /></label>
          <label>Name <input bind:value={editing.fields.name} required placeholder="Identification and authentication" /></label>
        </div>
        <label class="choice">
          <input type="checkbox" checked={editing.fields.pages !== null} onchange={(event) => setPages(event.currentTarget.checked)} disabled={data.documents.length === 0} />
          The pages of the chapter (the analysis with AI reads them)
        </label>
        {#if editing.fields.pages}
          <div class="row">
            <label>
              Document
              <select bind:value={editing.fields.pages.document_id}>
                {#each data.documents as document (document.id)}
                  <option value={document.id}>{document.title}{document.part ? ` — ${document.part}` : ""}</option>
                {/each}
              </select>
            </label>
            <label>From page <input type="number" min="1" bind:value={editing.fields.pages.first_page} /></label>
            <label>To page <input type="number" min="1" bind:value={editing.fields.pages.last_page} /></label>
          </div>
        {/if}
        <div class="buttons">
          <button type="submit">Save</button>
          <button type="button" onclick={() => (editing = null)}>Cancel</button>
        </div>
      </form>
    </div>
  {/if}

  {#if settings}
    <div class="overlay" role="presentation">
      <form class="dialog" onsubmit={saveSettings}>
        <h2>Settings of the project</h2>
        <label>Name <input bind:value={settings.name} required /></label>
        <div class="row">
          <label>Version <input bind:value={settings.version} /></label>
          <label>Author <input bind:value={settings.author} placeholder="Head of the laboratory" /></label>
        </div>
        <label>Description <textarea rows="3" bind:value={settings.description}></textarea></label>
        <fieldset class="flat">
          <legend>Documents of the standard the project uses</legend>
          {#each documents as document (document.id)}
            <label class="choice">
              <input type="checkbox" bind:group={settings.documents} value={document.id} />
              {document.title}{document.part ? ` — ${document.part}` : ""}
              <span class="note">({document.page_count} pages)</span>
            </label>
          {:else}
            <p class="note">No document is loaded: load them in the tab Documents of standards.</p>
          {/each}
        </fieldset>
        <div class="buttons">
          <button type="submit">Save</button>
          <button type="button" onclick={() => (settings = null)}>Cancel</button>
        </div>
      </form>
    </div>
  {/if}

  {#if exporting}
    <div class="overlay" role="presentation">
      <div class="dialog">
        <h2>Export the project</h2>
        <p class="note">
          The file holds the schemes, their tiles, the links and the sources. The documents of the standard are named by
          title, version and checksum, so that the evaluator's own copies are found.
        </p>
        <label class="choice">
          <input type="checkbox" bind:checked={withPdfs} /> Carry the PDFs of the documents in the file
          <span class="note">(the laboratory decides whether its copies of the standard travel with the project)</span>
        </label>
        <div class="buttons">
          <button type="button" class="primary" onclick={exportNow}>Choose where to save…</button>
          <button type="button" onclick={() => (exporting = false)}>Cancel</button>
        </div>
      </div>
    </div>
  {/if}

  {#if reading}
    <ContentsPanel
      {data}
      ondone={(next) => {
        data = next;
        notice = "The chapters found are in the list as proposed schemes.";
      }}
      onclose={() => (reading = false)}
    />
  {/if}

  {#if analysing}
    <ChaptersPanel
      {data}
      ondone={(next) => {
        data = next;
        notice = "The proposals of the analysis are in the schemes, marked AI.";

        // The scheme shown is drawn anew with its proposals.
        const shown = open;
        open = null;
        queueMicrotask(() => (open = shown));
      }}
      onclose={() => (analysing = false)}
    />
  {/if}
{:else if problem}
  <p class="problem">{problem}</p>
{:else}
  <p class="note">Opening the project…</p>
{/if}
