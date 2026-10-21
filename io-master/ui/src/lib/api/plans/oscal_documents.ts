// The element "documents of OSCAL" of the view Evaluation plan: its
// commands and its data (src/views/plans/oscal_documents/).
//
// The types mirror src/views/plans/oscal_documents/view.rs.

import { invoke } from "@tauri-apps/api/core";

export type OscalModel = "catalog" | "profile" | "mapping-collection" | "assessment-plan" | "assessment-results";

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

export function get(): Promise<OscalDocumentView[]> {
  return invoke("oscal_documents_get");
}

/** Asks for a file in a dialog of the system; null when none was chosen. */
export function importFile(): Promise<OscalDocumentView | null> {
  return invoke("oscal_documents_import");
}

/** Asks where to write the document; false when no place was chosen. */
export function exportFile(uuid: string, fileName: string): Promise<boolean> {
  return invoke("oscal_documents_export", { uuid, fileName });
}

export function remove(uuid: string): Promise<void> {
  return invoke("oscal_documents_delete", { uuid });
}
