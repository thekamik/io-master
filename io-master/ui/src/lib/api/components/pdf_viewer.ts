// The component "viewer of a standard": its commands and its data
// (src/components/pdf_viewer/).
//
// The types mirror src/components/pdf_viewer/view.rs.

import { invoke } from "@tauri-apps/api/core";

/** A document the viewer can show. */
export interface ViewerDocumentView {
  id: number;
  title: string;
  part: string;
  page_count: number;
}

export function documents(): Promise<ViewerDocumentView[]> {
  return invoke("pdf_viewer_documents");
}

/** The PDF of a document, as bytes. */
export function pdf(id: number): Promise<ArrayBuffer> {
  return invoke("pdf_viewer_get", { id });
}
