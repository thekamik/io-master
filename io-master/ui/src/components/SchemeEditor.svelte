<!--
  The drawing module of one scheme of a test project (steps 19 and 26
  of docs/todo.md): the tiles on a canvas (Svelte Flow), coloured by
  kind, joined by arrows; the tile chosen is edited in the panel on
  the right (TilePanel.svelte), the findings of the checks are listed
  there when no tile is chosen.

  - Tiles are added with the buttons above the canvas, moved by
    dragging, linked by dragging from the handle on the right of a
    tile to another tile.
  - Tiles of other schemes of the project linked to this one are drawn
    as shadows (ShadowNode.svelte): read only, those that lead into
    this scheme on the left, the others on the right; a click on one
    opens its scheme. A link to a tile of another scheme is made in the
    panel of a tile, or by dragging to a shadow already shown.
  - The program checks every change by the rules of a project
    (src/evaluation/scheme.rs) and answers with the whole scheme as
    stored, so the canvas always shows what the program keeps.
  - "Undo" takes the last change in the project back (also a whole
    analysis, a moved tile).
  - The sources of a tile are shown highlighted in the original page
    of the standard (PdfViewer.svelte).
-->
<script lang="ts">
  import { onMount } from "svelte";
  import {
    Background,
    Controls,
    MarkerType,
    MiniMap,
    SvelteFlow,
    type Connection,
    type Edge,
    type Node,
  } from "@xyflow/svelte";
  import "@xyflow/svelte/dist/style.css";

  import { sentence } from "../lib/api/errors";
import { acceptTiles, addLink, addTile, deleteLink, deleteTiles, moveTile, moveTiles, openScheme, schemeName, undoScheme, updateTile, type LinkView, type SchemeDataView, type ShadowView, type SourceView, type TileFields, type TileKind } from "../lib/api/projects";
  import AnalysisPanel from "./AnalysisPanel.svelte";
  import PdfViewer from "./PdfViewer.svelte";
  import ShadowNode from "./ShadowNode.svelte";
  import TileNode from "./TileNode.svelte";
  import TilePanel from "./TilePanel.svelte";

  let {
    schemeId,
    onchange,
    onopen,
  }: {
    schemeId: number;
    /** After every change: the list of the project shows the new counts. */
    onchange: () => void;
    /** Opens another scheme of the project (a click on a shadow). */
    onopen: (scheme: number) => void;
  } = $props();

  let data = $state<SchemeDataView | null>(null);
  let problem = $state("");
  let notice = $state("");

  let nodes = $state.raw<Node[]>([]);
  let edges = $state.raw<Edge[]>([]);

  let selected: number | null = $state(null);
  let selectedLink: number | null = $state(null);

  let filterText = $state("");
  let filterKind: "" | TileKind = $state("");

  let viewer = $state<{ documentId: number; page: number; quote: string; picking: boolean } | null>(null);
  let analysing = $state(false);

  let panel: ReturnType<typeof TilePanel> | undefined = $state();

  /** Changed after a change of many tiles (an analysis, Undo): the canvas is drawn anew and fitted to them. */
  let layout = $state(0);

  const nodeTypes = { tile: TileNode, shadow: ShadowNode };

  let tile = $derived(data?.tiles.find((candidate) => candidate.id === selected) ?? null);
  let proposed = $derived(data?.tiles.filter((candidate) => candidate.state === "proposed") ?? []);

  /** Whether a tile matches the filter; the others are dimmed. */
  function matches(candidate: { kind: TileKind; identifier: string; title: string }): boolean {
    const text = filterText.trim().toLowerCase();

    return (
      (!filterKind || candidate.kind === filterKind) &&
      (!text || `${candidate.identifier} ${candidate.title}`.toLowerCase().includes(text))
    );
  }

  /** The canvas from the scheme as the program stores it. */
  function show(next: SchemeDataView) {
    data = next;

    if (selected !== null && !next.tiles.some((candidate) => candidate.id === selected)) {
      selected = null;
    }

    const tiles: Node[] = next.tiles.map((candidate) => ({
      id: String(candidate.id),
      type: "tile",
      position: { x: candidate.x, y: candidate.y },
      data: {
        tile: candidate,
        findings: next.findings.filter((finding) => finding.tile === candidate.id).length,
      },
      selected: candidate.id === selected,
      class: matches(candidate) ? "" : "dimmed",
    }));

    // The shadows: those that lead into this scheme left of the
    // inputs, the others right of the outputs; not stored, not moved.
    const top = Math.min(0, ...next.tiles.map((candidate) => candidate.y));
    let left = 0;
    let right = 0;

    const shadows: Node[] = next.shadows.map((shadow) => {
      const leading = next.links.some((link) => link.from === shadow.id);
      const row = leading ? left++ : right++;

      return {
        id: String(shadow.id),
        type: "shadow",
        position: { x: leading ? -460 : 1300, y: top + row * 120 },
        data: { shadow },
        draggable: false,
        class: matches(shadow) ? "" : "dimmed",
      };
    });

    nodes = [...tiles, ...shadows];

    edges = next.links.map((link) => ({
      id: `l${link.id}`,
      source: String(link.from),
      target: String(link.to),
      class: `link ${link.meaning}`,
      markerEnd: { type: MarkerType.ArrowClosed },
      selected: link.id === selectedLink,
    }));
  }

  /** Runs a change and shows the scheme the program gives back. */
  async function act(work: () => Promise<SchemeDataView>, done = "") {
    problem = "";
    notice = "";

    try {
      show(await work());
      notice = done;
      onchange();
    } catch (error) {
      problem = sentence(error);

      // Back to what is stored (a refused link, a refused move).
      if (data) {
        show(data);
      }
    }
  }

  onMount(async () => {
    try {
      show(await openScheme(schemeId));
    } catch (error) {
      problem = sentence(error);
    }
  });

  $effect(() => {
    // The filter dims the tiles that do not match.
    void filterText;
    void filterKind;

    if (data) {
      show(data);
    }
  });

  /** A new tile of `kind`, below the others of its column. */
  function add(kind: TileKind) {
    if (!data) {
      return;
    }

    const column = { input: 0, requirement: 420, output: 840 }[kind];
    const lowest = Math.max(-140, ...data.tiles.filter((candidate) => candidate.kind === kind).map((candidate) => candidate.y));
    const titles = { input: "New input", requirement: "New requirement", output: "New output" };

    const fields: TileFields = {
      kind,
      identifier: "",
      title: titles[kind],
      description: "",
      place: "",
      state: "accepted",
      external: null,
      sections: kind === "output" ? data.sections.map((section) => section.name) : [],
      instructions: "",
      x: column,
      y: lowest + 140,
      sources: [],
    };

    act(async () => {
      const next = await addTile(schemeId, fields);
      selected = Math.max(...next.tiles.map((candidate) => candidate.id));
      return next;
    });
  }

  function connect(connection: Connection) {
    act(() => addLink(schemeId, Number(connection.source), Number(connection.target)));
  }

  function moved({ nodes: dragged }: { nodes: Node[] }) {
    for (const node of dragged) {
      if (node.type === "shadow") {
        continue;
      }

      const id = Number(node.id);
      const stored = data?.tiles.find((candidate) => candidate.id === id);

      // The canvas is drawn again from `data`: it keeps the new place.
      if (stored) {
        stored.x = node.position.x;
        stored.y = node.position.y;
      }

      moveTile(id, node.position.x, node.position.y).catch((error) => (problem = sentence(error)));
    }
  }

  function choose({ node }: { node: Node }) {
    if (node.type === "shadow") {
      onopen((node.data as { shadow: ShadowView }).shadow.scheme);
      return;
    }

    selected = Number(node.id);
    selectedLink = null;
  }

  function chooseLink({ edge }: { edge: Edge }) {
    selectedLink = Number(edge.id.slice(1));
    selected = null;
  }

  function save(fields: TileFields) {
    if (selected !== null) {
      const id = selected;
      act(() => updateTile(id, fields), "The tile was saved.");
    }
  }

  function removeLink(link: LinkView | number) {
    const id = typeof link === "number" ? link : link.id;
    selectedLink = null;
    act(() => deleteLink(schemeId, id));
  }

  /** Moves the chosen tile to another scheme of the project; its links stay. */
  function moveTo(scheme: number) {
    if (selected !== null) {
      const id = selected;
      selected = null;
      act(() => moveTiles(schemeId, [id], scheme), "The tile was moved; its links stay.").then(() => (layout += 1));
    }
  }

  function showSource(source: SourceView) {
    viewer = { documentId: source.document_id, page: source.page, quote: source.quote, picking: false };
  }

  function pickQuote() {
    const first = tile?.sources[0];
    const documentId = first?.document_id ?? data?.scheme.pages?.document_id ?? data?.documents[0]?.id;
    const page = first?.page ?? data?.scheme.pages?.first_page ?? 1;

    if (documentId !== undefined) {
      viewer = { documentId, page, quote: "", picking: true };
    }
  }

  function quoted(source: { document_id: number; page: number; quote: string }) {
    panel?.addSource(source);
    viewer = null;
    notice = "The quote was added to the tile; save the tile to keep it.";
  }
</script>

{#if data}
  <div class="editor">
    <header class="editor-head">
      <h3>{schemeName(data.scheme)}</h3>
      {#if data.scheme.state === "proposed"}<span class="badge">proposed by AI</span>{/if}
      {#if data.scheme.pages}
        <span class="note">pages {data.scheme.pages.first_page} to {data.scheme.pages.last_page}</span>
      {:else}
        <span class="note">no pages given</span>
      {/if}
    </header>

    <div class="toolbar">
      <button type="button" class="kind input" onclick={() => add("input")}>+ Input</button>
      <button type="button" class="kind requirement" onclick={() => add("requirement")}>+ Requirement</button>
      <button type="button" class="kind output" onclick={() => add("output")}>+ Output</button>
      <span class="separator"></span>
      <button type="button" class="primary" onclick={() => (analysing = true)}>Analyse with AI</button>
      {#if proposed.length > 0}
        <button type="button" onclick={() => act(() => acceptTiles(schemeId, proposed.map((candidate) => candidate.id)), "All proposals were accepted.")}>
          Accept all {proposed.length} proposals
        </button>
        <button type="button" class="danger" onclick={() => act(() => deleteTiles(schemeId, proposed.map((candidate) => candidate.id)), "All proposals were deleted.")}>
          Delete all proposals
        </button>
      {/if}
      <button
        type="button"
        onclick={async () => {
          await act(() => undoScheme(schemeId), "The last change was undone.");
          layout += 1;
        }}
        disabled={!data.can_undo}>Undo</button
      >
      {#if selectedLink !== null}
        <button type="button" class="danger" onclick={() => removeLink(selectedLink!)}>Remove the link</button>
      {/if}
      <span class="separator"></span>
      <input class="filter" placeholder="Find a tile" bind:value={filterText} />
      <select bind:value={filterKind}>
        <option value="">All kinds</option>
        <option value="input">Inputs</option>
        <option value="requirement">Requirements</option>
        <option value="output">Outputs</option>
      </select>
    </div>

    {#if problem}<p class="problem">{problem}</p>{/if}
    {#if notice}<p class="done">{notice}</p>{/if}

    <div class="editor-body">
      <div class="canvas">
        {#key layout}
        <SvelteFlow
          bind:nodes
          bind:edges
          {nodeTypes}
          fitView
          colorMode="system"
          deleteKey={null}
          minZoom={0.1}
          onconnect={connect}
          onnodedragstop={moved}
          onnodeclick={choose}
          onedgeclick={chooseLink}
          onpaneclick={() => {
            selected = null;
            selectedLink = null;
          }}
        >
          <Background />
          <Controls />
          <MiniMap pannable zoomable />
        </SvelteFlow>
        {/key}
        <div class="legend note" title="Link two tiles by dragging from the handle on the right of one to the other.">
          <span class="dot input"></span> input <span class="dot requirement"></span> requirement
          <span class="dot output"></span> output <span class="dot other"></span> other scheme
        </div>
      </div>

      <aside class="side">
        {#if tile}
          <TilePanel
            bind:this={panel}
            {data}
            {tile}
            onsave={save}
            onaccept={() => act(() => acceptTiles(schemeId, [tile!.id]), "The tile was accepted.")}
            ondelete={() => act(() => deleteTiles(schemeId, [tile!.id]), "The tile was deleted.")}
            ondeletelink={removeLink}
            onshow={showSource}
            onquote={pickQuote}
            onlink={(from, to) => act(() => addLink(schemeId, from, to), "The link was made.")}
            onmove={moveTo}
            {onopen}
          />
        {:else}
          <h3>Checks</h3>
          {#if data.findings.length === 0}
            <p class="done">No findings: every requirement here has its inputs and is reported.</p>
          {:else}
            <p class="note">{data.findings.length} findings. Choose one to open its tile.</p>
            <ul class="findings">
              {#each data.findings as finding, index (index)}
                {@const of = data.tiles.find((candidate) => candidate.id === finding.tile)}
                <li>
                  <button type="button" class="link" onclick={() => (selected = finding.tile)}>
                    <strong>{of?.identifier || of?.title}</strong>
                    <span class="note">{finding.message}</span>
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        {/if}
      </aside>
    </div>
  </div>

  {#if viewer}
    <PdfViewer
      documents={data.documents}
      documentId={viewer.documentId}
      page={viewer.page}
      quote={viewer.quote}
      onquote={viewer.picking ? quoted : undefined}
      onclose={() => (viewer = null)}
    />
  {/if}

  {#if analysing}
    <AnalysisPanel
      {data}
      ondone={(next) => {
        show(next);
        layout += 1;
        notice = "The proposals of the analysis are on the canvas, marked AI.";
        onchange();
      }}
      onclose={() => (analysing = false)}
    />
  {/if}
{:else if problem}
  <p class="problem">{problem}</p>
{:else}
  <p class="note">Opening the scheme…</p>
{/if}
