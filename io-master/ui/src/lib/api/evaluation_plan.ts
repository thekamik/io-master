// The commands of the view Evaluation plan, one function per command of its
// entry in the program (src/views/evaluation_plan/evaluation_plan.rs),
// named as the command: `oscalDocumentsGet` calls `oscal_documents_get`.
// The view works only on the documents of OSCAL; they are made from the PDFs
// of a standard in the view Converter (converter.ts).
//
// The types mirror the view.rs of the elements (src/views/evaluation_plan/);
// a change there needs the same change here.

import { invoke } from "@tauri-apps/api/core";

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

// ── When the view opens ──────────────────────────────────────────────

/** Called by OscalDocuments.svelte, when the view opens and after a change. */
export function oscalDocumentsGet(): Promise<OscalDocumentView[]> {
  return invoke("oscal_documents_get");
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
