// The element "documents of standards" of the view Evaluation plan: its
// commands and its data (src/views/evaluation_plan/standard_documents/).
//
// The types mirror src/views/evaluation_plan/standard_documents/view.rs.

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

/** An event of loading documents. */
export type LoadEvent =
  | { event: "file"; name: string; index: number; count: number }
  | { event: "pages"; done: number; total: number }
  | { event: "loaded"; document: DocumentView }
  | { event: "failed"; name: string; message: string }
  | { event: "end"; stopped: boolean };

export function get(): Promise<DocumentView[]> {
  return invoke("standard_documents_get");
}

/** Opens the dialog of the system for PDFs and loads them; the promise ends with the loading. */
export function load(onEvent: (event: LoadEvent) => void): Promise<void> {
  const channel = new Channel<LoadEvent>();

  channel.onmessage = onEvent;

  return invoke("standard_documents_load", { onEvent: channel });
}

/** Stops the loading. */
export function stop(): Promise<void> {
  return invoke("standard_documents_stop");
}

export function update(id: number, document: DocumentFields): Promise<DocumentView> {
  return invoke("standard_documents_update", { id, document });
}

export function remove(id: number): Promise<void> {
  return invoke("standard_documents_delete", { id });
}
