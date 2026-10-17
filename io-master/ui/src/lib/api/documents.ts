// The documents of standards (serializer/standard_view.rs;
// tauri_ui/commands/documents.rs).
//
// The types mirror the views of src/tauri_ui/serializer/; a change
// there needs the same change here.

import { Channel, invoke } from "@tauri-apps/api/core";

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

/** An event of loading documents (also of an import). */
export type LoadEvent =
  | { event: "file"; name: string; index: number; count: number }
  | { event: "pages"; done: number; total: number }
  | { event: "loaded"; document: DocumentView }
  | { event: "failed"; name: string; message: string }
  | { event: "end"; stopped: boolean };

export interface HitView {
  document_id: number;
  page: number;
  snippet: string;
}

export function listDocuments(): Promise<DocumentView[]> {
  return invoke("list_documents");
}

/** Opens the dialog of the system for PDFs and loads them; the promise ends with the loading. */
export function loadDocuments(onEvent: (event: LoadEvent) => void): Promise<void> {
  const channel = new Channel<LoadEvent>();

  channel.onmessage = onEvent;

  return invoke("load_documents", { onEvent: channel });
}

/** Stops the long work running (loading, an analysis). */
export function stopWork(): Promise<void> {
  return invoke("stop_work");
}

export function updateDocument(id: number, document: DocumentFields): Promise<DocumentView> {
  return invoke("update_document", { id, document });
}

export function deleteDocument(id: number): Promise<void> {
  return invoke("delete_document", { id });
}

export function documentPage(id: number, page: number): Promise<string> {
  return invoke("document_page", { id, page });
}

export function searchDocuments(text: string, document: number | null): Promise<HitView[]> {
  return invoke("search_documents", { text, document });
}

/** The PDF of a document, as bytes, for the viewer. */
export function documentPdf(id: number): Promise<ArrayBuffer> {
  return invoke("document_pdf", { id });
}
