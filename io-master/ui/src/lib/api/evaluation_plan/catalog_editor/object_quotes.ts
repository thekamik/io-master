// The element "quotes" of the editor of a catalog: its commands and its
// data (src/views/evaluation_plan/catalog_editor/object_quotes/).

import { invoke } from "@tauri-apps/api/core";

export interface QuoteView {
  id: number;
  source_document_id: number;
  /** The title and part of the document quoted. */
  document: string;
  page: number;
  text: string;
  /** Whether the text was found on its page. */
  found: boolean;
}

export interface ObjectQuotesView {
  quotes: QuoteView[];
  /** The document the viewer opens for a new quote; null: none loaded. */
  first_source: number | null;
}

export function get(uuid: string, objectId: string): Promise<ObjectQuotesView> {
  return invoke("object_quotes_get", { uuid, objectId });
}

export function add(uuid: string, objectId: string, source: number, page: number, text: string): Promise<void> {
  return invoke("object_quotes_add", { uuid, objectId, source, page, text });
}

export function remove(id: number): Promise<void> {
  return invoke("object_quotes_delete", { id });
}
