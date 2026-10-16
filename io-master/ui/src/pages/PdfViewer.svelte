<!--
  The viewer of a standard (step 18 of docs/todo.md): the original
  page of a PDF, with a quote highlighted in it, so that every source
  of a tile, also one the AI proposed, is checked against the
  standard itself.

  With `onquote`, text selected in the page can be taken as a quote
  for a tile (document, page and the selected text).

  The PDF comes from the encrypted database of the program
  (documentPdf) and is drawn by pdf.js, built into the window.
-->
<script lang="ts">
  import { onMount, tick } from "svelte";
  import type { PDFDocumentProxy } from "pdfjs-dist";

  import { documentPdf, sentence, type DocumentView } from "../lib/api";
  import { drawPage, highlight, openPdf, type DrawnPage } from "../lib/pdf";

  let {
    documents,
    documentId,
    page = 1,
    quote = "",
    onquote,
    onclose,
  }: {
    documents: DocumentView[];
    documentId: number;
    page?: number;
    quote?: string;
    onquote?: (source: { document_id: number; page: number; quote: string }) => void;
    onclose: () => void;
  } = $props();

  // The page shown; starts at the page asked for.
  // svelte-ignore state_referenced_locally
  let shown = $state({ document: documentId, page, quote });
  let scale = $state(1.3);
  let problem = $state("");
  let found: boolean | null = $state(null);
  let selected = $state("");
  let drawing = $state(false);

  let container: HTMLElement | undefined = $state();
  let drawn: DrawnPage | null = null;

  /** The open PDFs, by document: opened once. */
  const open = new Map<number, PDFDocumentProxy>();

  let current = $derived(documents.find((document) => document.id === shown.document));

  async function pdfOf(id: number): Promise<PDFDocumentProxy> {
    const known = open.get(id);

    if (known) {
      return known;
    }

    const pdf = await openPdf(await documentPdf(id));
    open.set(id, pdf);

    return pdf;
  }

  async function draw() {
    if (!container) {
      return;
    }

    drawing = true;
    problem = "";
    selected = "";

    try {
      const pdf = await pdfOf(shown.document);
      const number = Math.min(Math.max(1, shown.page), pdf.numPages);

      shown.page = number;
      drawn = await drawPage(pdf, number, scale, container);
      found = shown.quote ? highlight(drawn, shown.quote) : null;
    } catch (error) {
      problem = `The page cannot be shown: ${sentence(error)}`;
    } finally {
      drawing = false;
    }
  }

  onMount(() => {
    draw();

    return () => {
      for (const pdf of open.values()) {
        pdf.loadingTask.destroy();
      }
    };
  });

  async function go(change: Partial<typeof shown>) {
    shown = { ...shown, ...change };
    await tick();
    await draw();
  }

  function zoom(by: number) {
    scale = Math.min(3, Math.max(0.6, scale + by));
    draw();
  }

  /** The text selected inside the page. */
  function select() {
    const selection = window.getSelection();

    selected =
      selection && container && selection.anchorNode && container.contains(selection.anchorNode)
        ? selection.toString().trim()
        : "";
  }

  function take() {
    if (onquote && selected) {
      onquote({ document_id: shown.document, page: shown.page, quote: selected });
    }
  }

  function keydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      onclose();
    }
  }
</script>

<svelte:window onkeydown={keydown} onmouseup={select} />

<div class="overlay" role="presentation" onclick={(event) => event.target === event.currentTarget && onclose()}>
  <div class="viewer" role="dialog" aria-label="The viewer of a standard">
    <header class="viewer-bar">
      <select value={shown.document} onchange={(event) => go({ document: Number(event.currentTarget.value), page: 1, quote: "" })}>
        {#each documents as document (document.id)}
          <option value={document.id}>{document.title}{document.part ? ` — ${document.part}` : ""}</option>
        {/each}
      </select>

      <div class="buttons">
        <button type="button" onclick={() => go({ page: shown.page - 1, quote: "" })} disabled={drawing || shown.page <= 1}>‹</button>
        <span class="note">
          Page
          <input
            class="page-number"
            type="number"
            min="1"
            max={current?.page_count ?? 1}
            value={shown.page}
            onchange={(event) => go({ page: Number(event.currentTarget.value), quote: "" })}
          />
          of {current?.page_count ?? "?"}
        </span>
        <button
          type="button"
          onclick={() => go({ page: shown.page + 1, quote: "" })}
          disabled={drawing || shown.page >= (current?.page_count ?? 1)}>›</button
        >
        <button type="button" onclick={() => zoom(-0.2)}>−</button>
        <button type="button" onclick={() => zoom(0.2)}>+</button>
      </div>

      <button type="button" class="ghost" onclick={onclose}>Close</button>
    </header>

    {#if shown.quote}
      <p class={found === false ? "problem" : "note quote-line"}>
        {found === false
          ? "The quote is not found in the text of this page as pdf.js reads it; compare it with the page by eye."
          : `Highlighted: “${shown.quote}”`}
      </p>
    {/if}
    {#if problem}<p class="problem">{problem}</p>{/if}

    <div class="viewer-page">
      <div class="pdf-page" bind:this={container}></div>
    </div>

    {#if onquote}
      <footer class="viewer-bar">
        <span class="note">{selected ? `Selected: “${selected.slice(0, 120)}${selected.length > 120 ? "…" : ""}”` : "Select text in the page to quote it."}</span>
        <button type="button" class="primary" onclick={take} disabled={!selected}>Use as a quote of the tile</button>
      </footer>
    {/if}
  </div>
</div>
