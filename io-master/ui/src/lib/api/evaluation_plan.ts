// The commands of the view Evaluation plan, one function per command of its
// entry in the program (src/views/evaluation_plan/evaluation_plan.rs),
// named as the command: `oscalDocumentsGet` calls `oscal_documents_get`.
// The commands of the editor of a catalog are in evaluation_plan/catalog_editor/,
// those of the viewer of a standard in components/.
//
// The types mirror the view.rs of the elements (src/views/evaluation_plan/);
// a change there needs the same change here.

import { Channel, invoke } from "@tauri-apps/api/core";

// oscal_documents/view.rs
export type OscalModel =
  | "catalog"
  | "profile"
  | "mapping-collection"
  | "assessment-plan"
  | "assessment-results";

export interface OscalDocumentView {
  uuid: string;
  model: OscalModel;
  /** The name of the model for the evaluator ("Catalog", ...). */
  model_title: string;
  title: string;
  version: string;
  oscal_version: string;
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

/** Called by OscalDocuments.svelte, when the view opens and after a change. */
export function oscalDocumentsGet(): Promise<OscalDocumentView[]> {
  return invoke("oscal_documents_get");
}

/** Called by StandardDocuments.svelte, when its tab opens and after a change. */
export function standardDocumentsGet(): Promise<DocumentView[]> {
  return invoke("standard_documents_get");
}

// ── The actions of the elements ──────────────────────────────────────

/** Asks for a file in a dialog of the system; null when none was chosen. Called by OscalDocuments.svelte, "Import an OSCAL document". */
export function oscalDocumentsImport(): Promise<OscalDocumentView | null> {
  return invoke("oscal_documents_import");
}

/** Asks where to write the document; false when no place was chosen. Called by OscalDocuments.svelte, "Export". */
export function oscalDocumentsExport(
  uuid: string,
  fileName: string,
): Promise<boolean> {
  return invoke("oscal_documents_export", { uuid, fileName });
}

/** Called by OscalDocuments.svelte, "Delete". */
export function oscalDocumentsDelete(uuid: string): Promise<void> {
  return invoke("oscal_documents_delete", { uuid });
}

/** Called by NewCatalog.svelte, when its dialog opens. */
export function newCatalogSources(): Promise<SourceView[]> {
  return invoke("new_catalog_sources");
}

/** Called by NewCatalog.svelte, "Make". */
export function newCatalogCreate(
  title: string,
  version: string,
  sources: number[],
): Promise<OscalDocumentView> {
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
