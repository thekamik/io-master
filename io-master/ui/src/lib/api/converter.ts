// The commands of the view Converter, one function per command of its entry
// in the program (src/views/converter/converter.rs), named as the command:
// `catalogListGet` calls `catalog_list_get`. The commands of the editor of a
// catalog are in converter/catalog_editor.ts, those of the viewer of a
// standard in components/.
//
// The types mirror the view.rs of the elements (src/views/converter/); a
// change there needs the same change here.

import { Channel, invoke } from "@tauri-apps/api/core";

// catalog_list/view.rs
/** A catalog of OSCAL, opened in the editor of a catalog. */
export interface CatalogView {
  uuid: string;
  title: string;
  version: string;
  last_modified: string;
}

// standard_documents/view.rs
export interface DocumentView {
  id: number;
  title: string;
  part: string;
  version: string;
  file_name: string;
  sha256: string;
  page_count: number;
}

export interface DocumentFields {
  title: string;
  part: string;
  version: string;
}

/** An event of loading documents. */
export type LoadEvent =
  | { event: "file"; name: string; index: number; count: number }
  | { event: "pages"; done: number; total: number }
  | { event: "loaded"; document: DocumentView }
  | { event: "failed"; name: string; message: string }
  | { event: "end"; stopped: boolean };

// new_catalog/view.rs
/** A document a catalog can come from. */
export interface SourceView {
  id: number;
  title: string;
  part: string;
}

// document_search/view.rs
export interface HitView {
  document_id: number;
  /** The title of the document and its part. */
  document: string;
  page: number;
  snippet: string;
}

// ── When the view opens ──────────────────────────────────────────────

/** Called by CatalogList.svelte, when the view opens and after a change. */
export function catalogListGet(): Promise<CatalogView[]> {
  return invoke("catalog_list_get");
}

/** Called by StandardDocuments.svelte, when its tab opens and after a change. */
export function standardDocumentsGet(): Promise<DocumentView[]> {
  return invoke("standard_documents_get");
}

// ── The actions of the elements ──────────────────────────────────────

/** Called by NewCatalog.svelte, when its dialog opens. */
export function newCatalogSources(): Promise<SourceView[]> {
  return invoke("new_catalog_sources");
}

/** Called by NewCatalog.svelte, "Make". */
export function newCatalogCreate(
  title: string,
  version: string,
  sources: number[],
): Promise<CatalogView> {
  return invoke("new_catalog_create", { title, version, sources });
}

/** Opens the dialog of the system for PDFs and loads them; the promise ends with the loading. Called by StandardDocuments.svelte, "Load PDFs". */
export function standardDocumentsLoad(
  onEvent: (event: LoadEvent) => void,
): Promise<void> {
  const channel = new Channel<LoadEvent>();

  channel.onmessage = onEvent;

  return invoke("standard_documents_load", { onEvent: channel });
}

/** Stops the loading. Called by StandardDocuments.svelte, "Stop". */
export function standardDocumentsStop(): Promise<void> {
  return invoke("standard_documents_stop");
}

/** Called by StandardDocuments.svelte, "Save". */
export function standardDocumentsUpdate(
  id: number,
  document: DocumentFields,
): Promise<DocumentView> {
  return invoke("standard_documents_update", { id, document });
}

/** Called by StandardDocuments.svelte, "Delete". */
export function standardDocumentsDelete(id: number): Promise<void> {
  return invoke("standard_documents_delete", { id });
}

/** Called by DocumentSearch.svelte, "Search". */
export function documentSearchRun(
  text: string,
  document: number | null,
): Promise<HitView[]> {
  return invoke("document_search_run", { text, document });
}
