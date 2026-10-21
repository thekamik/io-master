<!--
  The editor of a catalog of OSCAL (step 41.3, docs/oscal.md): a
  standard made in the program from its documents (PDF), group by
  group and control by control.

  - On the left the tree of the catalog: its groups and controls.
  - In the middle the object chosen: its parts as the standard reads,
    its own fields as JSON of OSCAL (edited and saved; what it holds,
    its groups and controls, stays), its quotes checked on their pages.
  - On the right the AI: it reads a table of contents and proposes
    the groups, or the pages of a group and proposes its controls.
    Nothing it proposes enters the catalog until the evaluator accepts
    it, as it is or corrected (P1).

  The catalog lives in the core: every operation gives back the whole
  catalog as stored, and the page shows only that.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import {
    acceptProposal,
    addControl,
    addGroup,
    addQuote,
    catalogInstructions,
    deleteObject,
    deleteQuote,
    openCatalog,
    proposeControls,
    proposeGroups,
    rejectProposal,
    saveCatalogInstructions,
    updateControl,
    updateGroup,
    type BuildEvent,
    type CatalogTask,
    type CatalogView,
    type OscalControl,
    type OscalGroup,
    type OscalPart,
    type Place,
      stopWork,
} from "../lib/api/catalogs";
  import * as standardDocuments from "../lib/api/plans/standard_documents";
  import type { DocumentView } from "../lib/api/plans/standard_documents";
  import { sentence } from "../lib/api/errors";
  import PdfViewer from "../views/plans/PdfViewer.svelte";

  let { uuid, onclose }: { uuid: string; onclose: () => void } = $props();

  // The documents of the standard, for the work of the AI and the titles
  // of the sources. Until the editor moves to views/ (step 42) it reads
  // them with the command of the list of documents.
  let documents: DocumentView[] = $state([]);

  /** An object of the catalog found by its id. */
  type Found = { kind: "group"; object: OscalGroup } | { kind: "control"; object: OscalControl };

  let view = $state<CatalogView | null>(null);
  let problem = $state("");
  let notice = $state("");

  // The object chosen in the tree, and its own fields as JSON.
  let selected = $state<string | null>(null);
  let fields = $state("");

  // A new object: its kind, place and JSON.
  let adding = $state<{ kind: "group" | "control"; place: Place; text: string } | null>(null);

  // The viewer of a standard: a quote shown, or a new quote taken for `quoting`.
  let viewer = $state<{ documentId: number; page: number; quote: string } | null>(null);
  let quoting = $state<string | null>(null);

  // The work of the AI.
  let building = $state(false);
  let progress = $state("");
  let log: { done: boolean; text: string }[] = $state([]);
  let outline = $state({ document_id: 0, first_page: 1, last_page: 1 });
  let section = $state({ document_id: 0, first_page: 1, last_page: 1, partSize: 12000 });
  let correcting = $state<{ id: number; text: string } | null>(null);
  let instructions = $state<{ which: CatalogTask; text: string } | null>(null);

  let found = $derived(view && selected ? find(view.catalog.groups ?? [], view.catalog.controls ?? [], selected) : null);
  let quotes = $derived(view && selected ? view.quotes.filter((quote) => quote.object_id === selected) : []);
  let firstDocument = $derived(documents[0]?.id ?? 0);

  onMount(async () => {
    try {
      documents = await standardDocuments.get();
      outline.document_id = firstDocument;
      section.document_id = firstDocument;
      view = await openCatalog(uuid);
    } catch (error) {
      problem = sentence(error);
    }
  });

  /** Runs one operation that gives back the catalog, and shows it. */
  async function run(operation: () => Promise<CatalogView>, done = ""): Promise<boolean> {
    problem = "";
    notice = "";

    try {
      view = await operation();
      notice = done;
      return true;
    } catch (error) {
      problem = sentence(error);
      return false;
    }
  }

  function find(groups: OscalGroup[], controls: OscalControl[], id: string): Found | null {
    for (const control of controls) {
      if (control.id === id) {
        return { kind: "control", object: control };
      }

      const inner = find([], control.controls ?? [], id);
      if (inner) {
        return inner;
      }
    }

    for (const group of groups) {
      if (group.id === id) {
        return { kind: "group", object: group };
      }

      const inner = find(group.groups ?? [], group.controls ?? [], id);
      if (inner) {
        return inner;
      }
    }

    return null;
  }

  /** The own fields of an object: without the groups and controls it holds. */
  function ownFields(object: OscalGroup | OscalControl): string {
    const { groups: _groups, controls: _controls, ...own } = object as OscalGroup;
    return JSON.stringify(own, null, 2);
  }

  function choose(id: string) {
    selected = id;
    const chosen = view ? find(view.catalog.groups ?? [], view.catalog.controls ?? [], id) : null;
    fields = chosen ? ownFields(chosen.object) : "";
  }

  /** The text of the JSON the evaluator wrote, or null and a problem. */
  function parsed(text: string): unknown | null {
    try {
      return JSON.parse(text);
    } catch (error) {
      problem = `The text is not JSON: ${error instanceof Error ? error.message : String(error)}`;
      return null;
    }
  }

  async function saveFields() {
    if (!found || !selected) {
      return;
    }

    const object = parsed(fields);
    if (object === null) {
      return;
    }

    const id = selected;
    const kind = found.kind;
    const saved = await run(() => (kind === "group" ? updateGroup(uuid, id, object) : updateControl(uuid, id, object)), "Saved.");

    if (saved) {
      const newId = (object as { id?: string }).id ?? id;
      choose(newId);
    }
  }

  async function remove() {
    if (selected) {
      const id = selected;
      if (await run(() => deleteObject(uuid, id), `Deleted: ${id}.`)) {
        selected = null;
      }
    }
  }

  function startAdding(kind: "group" | "control", place: Place) {
    const template =
      kind === "group"
        ? { id: "", title: "" }
        : { id: "", title: "", parts: [{ name: "statement", prose: "" }] };
    adding = { kind, place, text: JSON.stringify(template, null, 2) };
  }

  async function add(event: SubmitEvent) {
    event.preventDefault();

    if (!adding) {
      return;
    }

    const object = parsed(adding.text);
    if (object === null) {
      return;
    }

    const { kind, place } = adding;
    const added = await run(() => (kind === "group" ? addGroup(uuid, place, object) : addControl(uuid, place, object)), "Added.");

    if (added) {
      adding = null;
      const id = (object as { id?: string }).id;
      if (id) {
        choose(id);
      }
    }
  }

  function takeQuote(source: { document_id: number; page: number; quote: string }) {
    const object = quoting;
    viewer = null;
    quoting = null;

    if (object) {
      run(() => addQuote(uuid, object, source.document_id, source.page, source.quote), "The quote was added.");
    }
  }

  function documentTitle(id: number): string {
    const document = documents.find((candidate) => candidate.id === id);
    return document ? `${document.title}${document.part ? ` — ${document.part}` : ""}` : "?";
  }

  function buildEvent(event: BuildEvent) {
    switch (event.event) {
      case "part":
        progress = `Part ${event.index} of ${event.count}: ${event.document}, pages ${event.first_page}–${event.last_page}…`;
        break;
      case "writing":
        progress = progress.replace(/ — .* characters$/, "") + ` — ${event.characters} characters`;
        break;
      case "proposed":
        log.push({ done: true, text: `Proposed: ${event.added}.` });
        for (const dropped of event.dropped) {
          log.push({ done: false, text: `Dropped: ${dropped}` });
        }
        break;
      case "failed":
        log.push({ done: false, text: event.message });
        break;
      case "end":
        progress = event.stopped ? "Stopped." : "";
        break;
    }
  }

  async function build(work: () => Promise<CatalogView>) {
    building = true;
    log = [];
    progress = "";
    await run(work);
    building = false;
  }

  function buildGroups(event: SubmitEvent) {
    event.preventDefault();
    const range = { ...outline };
    build(() => proposeGroups(uuid, range, buildEvent));
  }

  function buildControls(event: SubmitEvent) {
    event.preventDefault();
    const group = found?.kind === "group" ? (selected ?? null) : null;
    const { partSize, ...range } = section;
    build(() => proposeControls(uuid, group, [range], partSize, buildEvent));
  }

  async function accept(id: number, corrected: unknown | null) {
    if (await run(() => acceptProposal(uuid, id, corrected), "The proposal is in the catalog.")) {
      correcting = null;
    }
  }

  function acceptCorrected() {
    if (correcting) {
      const object = parsed(correcting.text);
      if (object !== null) {
        accept(correcting.id, object);
      }
    }
  }

  async function openInstructions(which: CatalogTask) {
    try {
      instructions = { which, text: await catalogInstructions(which) };
    } catch (error) {
      problem = sentence(error);
    }
  }

  async function saveInstructions(text: string) {
    if (instructions) {
      try {
        instructions.text = await saveCatalogInstructions(instructions.which, text);
        notice = "The instructions were saved.";
      } catch (error) {
        problem = sentence(error);
      }
    }
  }
</script>

{#snippet tree(groups: OscalGroup[], controls: OscalControl[])}
  <ul class="catalog-tree">
    {#each controls as control (control.id)}
      <li class:active={selected === control.id}>
        <button type="button" class="link" onclick={() => choose(control.id)}>
          <span class="object-id">{control.id}</span> {control.title}
        </button>
        {#if control.controls?.length}{@render tree([], control.controls)}{/if}
      </li>
    {/each}
    {#each groups as group, index (group.id ?? index)}
      <li class:active={selected !== null && selected === group.id}>
        <button type="button" class="link group" onclick={() => group.id && choose(group.id)}>
          <span class="object-id">{group.id ?? ""}</span> {group.title}
        </button>
        {@render tree(group.groups ?? [], group.controls ?? [])}
      </li>
    {/each}
  </ul>
{/snippet}

{#snippet parts(list: OscalPart[])}
  {#each list as part, index (part.id ?? index)}
    <div class="oscal-part">
      <span class="note">{part.name}{part.id ? ` · ${part.id}` : ""}{part.title ? ` · ${part.title}` : ""}</span>
      {#if part.prose}<p>{part.prose}</p>{/if}
      {#if part.parts?.length}{@render parts(part.parts)}{/if}
    </div>
  {/each}
{/snippet}

<div class="editor-head">
  <button type="button" class="small" onclick={onclose}>← All OSCAL documents</button>
  {#if view}
    <h2>{view.catalog.metadata.title}</h2>
    <span class="note">{view.catalog.metadata.version} · last modified {view.document.last_modified}</span>
  {/if}
</div>

{#if problem}<p class="problem">{problem}</p>{/if}
{#if notice}<p class="done">{notice}</p>{/if}

{#if view}
  <div class="catalog-body">
    <div class="card catalog-side">
      <div class="buttons">
        <button type="button" class="small" onclick={() => startAdding("group", { kind: "top" })}>+ Group</button>
        <button type="button" class="small" onclick={() => startAdding("control", { kind: "top" })}>+ Control</button>
      </div>
      {#if !view.catalog.groups?.length && !view.catalog.controls?.length}
        <p class="note">The catalog is empty. Add a group, or let the AI read the table of contents.</p>
      {/if}
      {@render tree(view.catalog.groups ?? [], view.catalog.controls ?? [])}
    </div>

    <div class="card catalog-object">
      {#if found && selected}
        <div class="panel-head">
          <h3>{found.kind === "group" ? "Group" : "Control"} {selected}</h3>
          <div class="buttons">
            {#if found.kind === "group"}
              <button type="button" class="small" onclick={() => startAdding("group", { kind: "group", id: selected ?? "" })}>+ Group in it</button>
              <button type="button" class="small" onclick={() => startAdding("control", { kind: "group", id: selected ?? "" })}>+ Control in it</button>
            {:else}
              <button type="button" class="small" onclick={() => startAdding("control", { kind: "control", id: selected ?? "" })}>+ Enhancement</button>
            {/if}
            <button type="button" class="small danger" onclick={remove}>Delete with its content</button>
          </div>
        </div>
        <h4>{found.object.title}</h4>
        {@render parts(found.object.parts ?? [])}

        <h4>Quotes</h4>
        {#each quotes as quote (quote.id)}
          <div class="source">
            <div class="source-head">
              <button type="button" class="link" onclick={() => (viewer = { documentId: quote.source_document_id, page: quote.page, quote: quote.text })}>
                {documentTitle(quote.source_document_id)}, page {quote.page}
              </button>
              {#if !quote.found}<span class="tile-flag warn">not found on its page</span>{/if}
              <button type="button" class="small danger" onclick={() => run(() => deleteQuote(uuid, quote.id))}>Delete</button>
            </div>
            <blockquote>{quote.text}</blockquote>
          </div>
        {:else}
          <p class="note">No quote yet: the passage of the standard this object comes from.</p>
        {/each}
        <button
          type="button"
          class="small"
          disabled={documents.length === 0}
          onclick={() => {
            quoting = selected;
            viewer = { documentId: quotes[0]?.source_document_id ?? firstDocument, page: quotes[0]?.page ?? 1, quote: "" };
          }}>Add a quote from the standard…</button>

        <details class="settings">
          <summary>Its fields in OSCAL</summary>
          <textarea class="mono" rows="14" bind:value={fields}></textarea>
          <p class="note">
            The own fields of the object (JSON of OSCAL 1.2.3); the groups and controls it holds stay as they are. Every
            change is checked against the schema of NIST.
          </p>
          <div class="buttons"><button type="button" onclick={saveFields}>Save</button></div>
        </details>
      {:else}
        <p class="note">Choose a group or a control in the tree.</p>
      {/if}
    </div>

    <div class="card catalog-ai">
      <h3>The AI</h3>
      {#if documents.length === 0}
        <p class="note">Load the PDFs of the standard (tab Documents of standards) for the AI to read them.</p>
      {:else}
        <form onsubmit={buildGroups}>
          <h4>Groups from the table of contents</h4>
          <select bind:value={outline.document_id}>
            {#each documents as document (document.id)}<option value={document.id}>{documentTitle(document.id)}</option>{/each}
          </select>
          <div class="range">
            <label>Pages <input type="number" min="1" bind:value={outline.first_page} /></label>
            <label>to <input type="number" min="1" bind:value={outline.last_page} /></label>
          </div>
          <button type="submit" disabled={building}>Propose the groups</button>
        </form>

        <form onsubmit={buildControls}>
          <h4>Controls of {found?.kind === "group" ? `group ${selected}` : "the top of the catalog"}</h4>
          <select bind:value={section.document_id}>
            {#each documents as document (document.id)}<option value={document.id}>{documentTitle(document.id)}</option>{/each}
          </select>
          <div class="range">
            <label>Pages <input type="number" min="1" bind:value={section.first_page} /></label>
            <label>to <input type="number" min="1" bind:value={section.last_page} /></label>
          </div>
          <label>Characters per question <input type="number" min="2000" step="1000" bind:value={section.partSize} /></label>
          <button type="submit" disabled={building}>Propose the controls</button>
        </form>

        <div class="buttons">
          <button type="button" class="small" onclick={() => openInstructions("catalog_outline")}>Instructions: groups</button>
          <button type="button" class="small" onclick={() => openInstructions("catalog_section")}>Instructions: controls</button>
        </div>
      {/if}

      {#if building || progress || log.length > 0}
        <div class="load-log">
          {#if progress}<p class="note">{progress}</p>{/if}
          {#each log as line, index (index)}
            <div class={line.done ? "done" : "problem"}>{line.text}</div>
          {/each}
          {#if building}
            <button type="button" class="small danger" onclick={stopWork}>Stop</button>
          {:else}
            <button type="button" class="small" onclick={() => (log = [])}>Close</button>
          {/if}
        </div>
      {/if}

      <h4>Proposals ({view.proposals.length})</h4>
      {#each view.proposals as proposal (proposal.id)}
        <div class="proposal">
          <div class="note">
            {proposal.kind === "group" ? "Group" : "Control"} in {proposal.parent_id ?? "the top"} · {proposal.model}
          </div>
          <strong><span class="object-id">{proposal.content.id ?? ""}</span> {proposal.content.title}</strong>
          {@render parts(proposal.content.parts ?? [])}
          {#each proposal.quotes as quote, index (index)}
            <div class="source">
              <button type="button" class="link" onclick={() => (viewer = { documentId: quote.source_document_id, page: quote.page, quote: quote.text })}>
                {documentTitle(quote.source_document_id)}, page {quote.page}
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
              <button type="button" class="small" onclick={() => (correcting = { id: proposal.id, text: JSON.stringify(proposal.content, null, 2) })}>Correct…</button>
              <button type="button" class="small danger" onclick={() => run(() => rejectProposal(uuid, proposal.id))}>Reject</button>
            </div>
          {/if}
        </div>
      {:else}
        <p class="note">No proposal waits for you.</p>
      {/each}
    </div>
  </div>
{/if}

{#if adding}
  <div class="overlay" role="presentation">
    <form class="dialog" onsubmit={add}>
      <h2>A new {adding.kind}</h2>
      <p class="note">
        In {adding.place.kind === "top" ? "the top of the catalog" : adding.place.id}. JSON of OSCAL 1.2.3: an id (for CC,
        "FAU_GEN.1"), a title, and for a control its statement.
      </p>
      <textarea class="mono" rows="12" bind:value={adding.text}></textarea>
      <div class="buttons">
        <button type="submit">Add</button>
        <button type="button" onclick={() => (adding = null)}>Cancel</button>
      </div>
    </form>
  </div>
{/if}

{#if instructions}
  {@const current = instructions}
  <div class="overlay" role="presentation">
    <div class="dialog wide">
      <h2>Instructions to the AI: {current.which === "catalog_outline" ? "groups" : "controls"}</h2>
      <p class="note">Evaluation support, kept in your database. Saving an empty text brings back the default ones.</p>
      <textarea class="mono" rows="18" bind:value={current.text}></textarea>
      <div class="buttons">
        <button type="button" onclick={() => saveInstructions(current.text)}>Save</button>
        <button type="button" onclick={() => saveInstructions("")}>Default</button>
        <button type="button" onclick={() => (instructions = null)}>Close</button>
      </div>
    </div>
  </div>
{/if}

{#if viewer}
  <PdfViewer
    documentId={viewer.documentId}
    page={viewer.page}
    quote={viewer.quote}
    onquote={quoting ? takeQuote : undefined}
    onclose={() => {
      viewer = null;
      quoting = null;
    }}
  />
{/if}
