// The commands of the component "viewer of a standard", one function per
// command of its entry in the program (src/components/pdf_viewer/pdf_viewer.rs),
// named as the command: `pdfViewerGet` calls `pdf_viewer_get`.
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

/** The documents the viewer can show. Called by PdfViewer.svelte when it opens. */
export function pdfViewerDocuments(): Promise<ViewerDocumentView[]> {
  return invoke("pdf_viewer_documents");
}

/** The PDF of a document, as bytes. Called by PdfViewer.svelte when it opens and when another document is chosen. */
export function pdfViewerGet(id: number): Promise<ArrayBuffer> {
  return invoke("pdf_viewer_get", { id });
}
