<!--
  The fields of the tile chosen on the canvas (SchemeEditor.svelte):
  what it is, where in the standard it is, its sources (each shown
  highlighted in the viewer), for an input whether it is described in
  another document, for an output the sections of its report; its
  links (also to tiles of other schemes of the project, made here: a
  requirement of another chapter that must be met first, an output of
  another chapter that reports this one); moving the tile to another
  scheme; and the actions: save, accept a proposal of the AI, delete.

  The panel works on a copy: nothing is stored before "Save".
-->
<script lang="ts">
  import {
    openScheme,
    schemeName,
    sentence,
    type FindingView,
    type LinkView,
    type SchemeDataView,
    type SourceView,
    type TileFields,
    type TileView,
  } from "../lib/api";

  let {
    data,
    tile,
    onsave,
    onaccept,
    ondelete,
    ondeletelink,
    onshow,
    onquote,
    onlink,
    onmove,
    onopen,
  }: {
    data: SchemeDataView;
    tile: TileView;
    onsave: (fields: TileFields) => void;
    onaccept: () => void;
    ondelete: () => void;
    ondeletelink: (link: LinkView) => void;
    onshow: (source: SourceView) => void;
    /** Opens the viewer to select a quote; the panel gets it through `addSource`. */
    onquote: () => void;
    /** Links two tiles of the project. */
    onlink: (from: number, to: number) => void;
    /** Moves the tile to another scheme. */
    onmove: (scheme: number) => void;
    /** Opens another scheme. */
    onopen: (scheme: number) => void;
  } = $props();

  /** The schemes of the project other than this one. */
  let others = $derived(data.schemes.filter((scheme) => scheme.id !== data.scheme.id));

  // A link to a tile of another scheme: the scheme, its tiles (read
  // when it is chosen), the tile and the direction.
  let linkScheme = $state<number | null>(null);
  let linkTiles = $state<TileView[]>([]);
  let linkTile = $state<number | null>(null);
  let linkOutgoing = $state(false);
  let linkProblem = $state("");

  let moveScheme = $state<number | null>(null);

  async function chooseLinkScheme() {
    linkTiles = [];
    linkTile = null;
    linkProblem = "";

    if (linkScheme !== null) {
      try {
        linkTiles = (await openScheme(linkScheme)).tiles;
      } catch (error) {
        linkProblem = sentence(error);
      }
    }
  }

  function makeLink() {
    if (linkTile !== null) {
      if (linkOutgoing) {
        onlink(tile.id, linkTile);
      } else {
        onlink(linkTile, tile.id);
      }

      linkTile = null;
    }
  }

  // A copy of the tile; a new one whenever another tile is chosen or
  // the stored one changes (the effect below fills it before the
  // panel is drawn).
  let fields: TileFields = $state({
    kind: "input",
    identifier: "",
    title: "",
    description: "",
    place: "",
    state: "accepted",
    external: null,
    sections: [],
    instructions: "",
    x: 0,
    y: 0,
    sources: [],
  });
  let external = $state(false);

  $effect.pre(() => {
    fields = structuredClone($state.snapshot(tile));
    external = tile.external !== null;
  });

  let findings: FindingView[] = $derived(data.findings.filter((finding) => finding.tile === tile.id));
  let linked = $derived(data.links.some((link) => link.from === tile.id || link.to === tile.id));

  const meanings = {
    needed_by: "is needed by",
    reported_in: "is reported in",
    provides: "provides",
    precedes: "must be met before",
  };

  /** The links of the tile, as sentences. */
  let links = $derived(
    data.links
      .filter((link) => link.from === tile.id || link.to === tile.id)
      .map((link) => {
        const id = link.from === tile.id ? link.to : link.from;
        const own = data.tiles.find((candidate) => candidate.id === id);
        const shadow = data.shadows.find((candidate) => candidate.id === id);
        const other = own ?? shadow;
        const name = other ? other.identifier || other.title : "?";
        const where = shadow ? ` (in ${schemeName({ number: shadow.scheme_number, name: shadow.scheme_name })})` : "";

        return {
          link,
          scheme: shadow?.scheme ?? null,
          text:
            link.from === tile.id
              ? `This ${meanings[link.meaning]} ${name}${where}`
              : `${name}${where} ${meanings[link.meaning]} this`,
        };
      }),
  );

  function documentName(id: number): string {
    const document = data.documents.find((candidate) => candidate.id === id);

    return document ? `${document.title}${document.part ? `, ${document.part}` : ""}` : "?";
  }

  /** A quote selected in the viewer, added to the copy (stored with "Save"). */
  export function addSource(source: { document_id: number; page: number; quote: string }) {
    fields.sources = [...fields.sources, { ...source, found: true }];
  }

  function removeSource(index: number) {
    fields.sources = fields.sources.filter((_, at) => at !== index);
  }

  function toggleSection(name: string, on: boolean) {
    fields.sections = on
      ? data.sections.map((section) => section.name).filter((known) => known === name || fields.sections.includes(known))
      : fields.sections.filter((known) => known !== name);
  }

  function kindChanged() {
    if (fields.kind === "output" && fields.sections.length === 0) {
      fields.sections = data.sections.map((section) => section.name);
    }

    if (fields.kind !== "output") {
      fields.sections = [];
      fields.instructions = "";
    }

    if (fields.kind !== "input") {
      external = false;
    }
  }

  function save(event: SubmitEvent) {
    event.preventDefault();

    onsave({
      ...fields,
      external: external && fields.kind === "input" ? (fields.external ?? { document: "", place: "" }) : null,
    });
  }
</script>

<form class="tile-panel" onsubmit={save}>
  <div class="panel-head">
    <h3>{tile.identifier || tile.title}</h3>
    {#if tile.state === "proposed"}<span class="badge">proposed by AI</span>{/if}
  </div>

  {#each findings as finding (finding.kind)}
    <p class="note finding">⚠ {finding.message}</p>
  {/each}

  <label>
    Kind
    <select bind:value={fields.kind} onchange={kindChanged} disabled={linked}>
      <option value="input">Input: what the evaluation needs</option>
      <option value="requirement">Requirement: what the standard imposes</option>
      <option value="output">Output: the result in the form of a report</option>
    </select>
    {#if linked}<span class="note">A linked tile keeps its kind; remove its links to change it.</span>{/if}
  </label>
  <label>Title <input bind:value={fields.title} required maxlength="300" /></label>
  <div class="row">
    <label>Identifier <input class="mono" bind:value={fields.identifier} placeholder="ADV_FSP.4.1E" /></label>
    <label>Place in the standard <input bind:value={fields.place} placeholder="Clause 5.3" /></label>
  </div>
  <label>Description <textarea rows="4" bind:value={fields.description}></textarea></label>

  {#if fields.kind === "input"}
    <label class="choice">
      <input type="checkbox" bind:checked={external} onchange={() => (fields.external ??= { document: "", place: "" })} />
      Described in another document of the standard (to be completed later)
    </label>
    {#if external && fields.external}
      <div class="row">
        <label>Document <input bind:value={fields.external.document} placeholder="CC Part 3" required /></label>
        <label>Place <input bind:value={fields.external.place} placeholder="ALC_FLR.2" /></label>
      </div>
    {/if}
  {/if}

  {#if fields.kind === "output"}
    <fieldset class="flat">
      <legend>Sections of the report</legend>
      {#each data.sections as section (section.name)}
        <label class="choice" title={section.description}>
          <input
            type="checkbox"
            checked={fields.sections.includes(section.name)}
            onchange={(event) => toggleSection(section.name, event.currentTarget.checked)}
          />
          {section.title}
        </label>
      {/each}
    </fieldset>
    <label>
      Instructions for the report
      <textarea rows="3" bind:value={fields.instructions} placeholder="What the laboratory expects written here"></textarea>
    </label>
  {/if}

  <fieldset class="flat">
    <legend>Sources in the standard</legend>
    {#each fields.sources as source, index (index)}
      <div class="source">
        <div class="source-head">
          <span class="note">{documentName(source.document_id)}, page {source.page}</span>
          {#if source.found}
            <span class="badge success">found</span>
          {:else}
            <span class="badge danger">not on the page</span>
          {/if}
        </div>
        <blockquote>{source.quote}</blockquote>
        <div class="buttons">
          <button type="button" class="small" onclick={() => onshow(source)}>Show in the standard</button>
          <button type="button" class="small danger" onclick={() => removeSource(index)}>Remove</button>
        </div>
      </div>
    {:else}
      <p class="note">No source yet.</p>
    {/each}
    <button type="button" class="small" onclick={onquote} disabled={data.documents.length === 0}>
      Add a quote from the standard
    </button>
  </fieldset>

  {#if links.length > 0}
    <fieldset class="flat">
      <legend>Links</legend>
      {#each links as { link, text, scheme } (link.id)}
        <div class="link-line">
          <span>{text}</span>
          {#if scheme !== null}
            <button type="button" class="small" onclick={() => onopen(scheme)}>Open</button>
          {/if}
          <button type="button" class="small danger" onclick={() => ondeletelink(link)}>Remove</button>
        </div>
      {/each}
    </fieldset>
  {/if}

  {#if others.length > 0}
    <details class="cross">
      <summary>Link to a tile of another scheme</summary>
      <label>
        Scheme
        <select bind:value={linkScheme} onchange={chooseLinkScheme}>
          <option value={null}>Choose a scheme</option>
          {#each others as scheme (scheme.id)}
            <option value={scheme.id}>{schemeName(scheme)}</option>
          {/each}
        </select>
      </label>
      {#if linkProblem}<p class="problem">{linkProblem}</p>{/if}
      {#if linkScheme !== null}
        <label>
          Tile
          <select bind:value={linkTile}>
            <option value={null}>Choose a tile</option>
            {#each linkTiles as candidate (candidate.id)}
              <option value={candidate.id}>{candidate.kind}: {candidate.identifier || candidate.title}</option>
            {/each}
          </select>
        </label>
        <label class="choice">
          <input type="radio" bind:group={linkOutgoing} value={false} /> from that tile to this one (it is needed first)
        </label>
        <label class="choice">
          <input type="radio" bind:group={linkOutgoing} value={true} /> from this tile to that one
        </label>
        <button type="button" class="small" onclick={makeLink} disabled={linkTile === null}>Make the link</button>
      {/if}
    </details>

    <details class="cross">
      <summary>Move to another scheme</summary>
      <label>
        Scheme
        <select bind:value={moveScheme}>
          <option value={null}>Choose a scheme</option>
          {#each others as scheme (scheme.id)}
            <option value={scheme.id}>{schemeName(scheme)}</option>
          {/each}
        </select>
      </label>
      <p class="note">The links of the tile stay; in this scheme it is then shown as a shadow.</p>
      <button type="button" class="small" onclick={() => moveScheme !== null && onmove(moveScheme)} disabled={moveScheme === null}>
        Move the tile
      </button>
    </details>
  {/if}

  <div class="buttons">
    <button type="submit">Save</button>
    {#if tile.state === "proposed"}<button type="button" onclick={onaccept}>Accept</button>{/if}
    <button type="button" class="danger" onclick={ondelete}>Delete</button>
  </div>
</form>
