// The element "search" of the view Evaluation plan: its command and its
// data (src/views/evaluation_plan/document_search/).
//
// The types mirror src/views/evaluation_plan/document_search/view.rs.

import { invoke } from "@tauri-apps/api/core";

export interface HitView {
  document_id: number;
  /** The title of the document and its part. */
  document: string;
  page: number;
  snippet: string;
}

export function run(text: string, document: number | null): Promise<HitView[]> {
  return invoke("document_search_run", { text, document });
}
