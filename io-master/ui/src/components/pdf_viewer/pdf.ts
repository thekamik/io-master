// pdf.js of Mozilla for the viewer of standards (step 18 of
// docs/history.md): the page of a PDF drawn on a canvas, with a layer of
// its text above it, in which a quote is highlighted and from which
// text is selected.
//
// Everything is built into the program: the worker, the maps of
// characters and the standard fonts (scripts/copy-pdfjs.mjs). Nothing
// is loaded from the network, and the Content-Security-Policy of the
// window lets no code be evaluated.

import type { PDFDocumentProxy } from "pdfjs-dist";
import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";

/**
 * pdf.js, loaded when the viewer is first opened: it is large, and the
 * rest of the window does not need it.
 */
async function pdfjs() {
  const library = await import("pdfjs-dist");

  library.GlobalWorkerOptions.workerSrc = workerUrl;

  return library;
}

/** Opens a PDF given as bytes. */
export async function openPdf(bytes: ArrayBuffer): Promise<PDFDocumentProxy> {
  const { getDocument } = await pdfjs();

  return getDocument({
    data: new Uint8Array(bytes),
    cMapUrl: "./pdfjs/cmaps/",
    cMapPacked: true,
    standardFontDataUrl: "./pdfjs/standard_fonts/",
    enableXfa: false,
  }).promise;
}

/**
 * The text of a page in the form of src/evaluation/quote.rs (lower
 * case, one space for every run of spaces, divided words joined), so
 * that a quote found by the program is found here too.
 */
export function normalize(text: string): string {
  return text
    .replace(/[­​﻿]/g, "")
    .replace(/[“”„«»]/g, '"')
    .replace(/[‘’‚]/g, "'")
    .replace(/[–—‐‑−]/g, "-")
    .replace(/-\n[ \t]*(?=\p{Ll})/gu, "")
    .replace(/\s+/g, " ")
    .trim()
    .toLowerCase();
}

/** A page drawn into `container`: a canvas and its text layer. */
export interface DrawnPage {
  /** The spans of the text layer, in reading order. */
  spans: HTMLElement[];
}

/** Draws page `number` of `pdf` at `scale` into `container`, replacing what was there. */
export async function drawPage(
  pdf: PDFDocumentProxy,
  number: number,
  scale: number,
  container: HTMLElement,
): Promise<DrawnPage> {
  const page = await pdf.getPage(number);
  const viewport = page.getViewport({ scale });
  const ratio = window.devicePixelRatio || 1;

  container.replaceChildren();
  container.style.setProperty("--total-scale-factor", String(scale));
  container.style.width = `${viewport.width}px`;
  container.style.height = `${viewport.height}px`;

  const canvas = document.createElement("canvas");
  canvas.width = Math.floor(viewport.width * ratio);
  canvas.height = Math.floor(viewport.height * ratio);
  canvas.style.width = `${viewport.width}px`;
  canvas.style.height = `${viewport.height}px`;
  container.append(canvas);

  const context = canvas.getContext("2d");

  if (context) {
    await page.render({
      canvas,
      canvasContext: context,
      viewport,
      transform: ratio === 1 ? undefined : [ratio, 0, 0, ratio, 0, 0],
    }).promise;
  }

  const layer = document.createElement("div");
  layer.className = "textLayer";
  container.append(layer);

  const { TextLayer } = await pdfjs();
  const text = new TextLayer({ textContentSource: page.streamTextContent(), container: layer, viewport });
  await text.render();

  return { spans: [...layer.querySelectorAll<HTMLElement>("span")] };
}

/**
 * Marks the spans of a drawn page that hold `quote` (in the form of
 * normalize()); returns whether it was found. The first marked span
 * is scrolled into view.
 */
export function highlight(page: DrawnPage, quote: string): boolean {
  for (const span of page.spans) {
    span.classList.remove("highlight");
  }

  const wanted = normalize(quote);

  if (!wanted) {
    return false;
  }

  // The text of the page joined from its spans, and where each span
  // starts in it.
  let joined = "";
  const starts: number[] = [];

  for (const span of page.spans) {
    const piece = normalize(span.textContent ?? "");

    if (!piece) {
      starts.push(joined.length);
      continue;
    }

    if (joined) {
      joined += " ";
    }

    starts.push(joined.length);
    joined += piece;
  }

  const at = joined.indexOf(wanted);

  if (at < 0) {
    return false;
  }

  const end = at + wanted.length;
  let first: HTMLElement | null = null;

  page.spans.forEach((span, index) => {
    const start = starts[index] ?? 0;
    const length = normalize(span.textContent ?? "").length;

    if (length > 0 && start < end && start + length > at) {
      span.classList.add("highlight");
      first ??= span;
    }
  });

  (first as HTMLElement | null)?.scrollIntoView({ block: "center", behavior: "smooth" });

  return true;
}
