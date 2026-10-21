// The element "new catalog" of the view Evaluation plan: its commands and
// its data (src/views/plans/new_catalog/).
//
// The types mirror src/views/plans/new_catalog/view.rs.

import { invoke } from "@tauri-apps/api/core";

import type { OscalDocumentView } from "./oscal_documents";

/** A document a catalog can come from. */
export interface SourceView {
  id: number;
  title: string;
  part: string;
}

export function sources(): Promise<SourceView[]> {
  return invoke("new_catalog_sources");
}

export function create(title: string, version: string, sources: number[]): Promise<OscalDocumentView> {
  return invoke("new_catalog_create", { title, version, sources });
}
