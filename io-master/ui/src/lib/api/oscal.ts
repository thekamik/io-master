// The documents of NIST OSCAL (serializer/oscal_view.rs;
// tauri_ui/commands/oscal.rs): catalogs, mappings, profiles,
// assessment plans and assessment results (docs/oscal.md).
//
// The types mirror the views of src/tauri_ui/serializer/; a change
// there needs the same change here.

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

export function listOscalDocuments(): Promise<OscalDocumentView[]> {
  return invoke("list_oscal_documents");
}

/** Asks for a file in a dialog of the system; null when none was chosen. */
export function importOscalDocument(): Promise<OscalDocumentView | null> {
  return invoke("import_oscal_document");
}

/** Asks where to write the document; false when no place was chosen. */
export function exportOscalDocument(uuid: string, fileName: string): Promise<boolean> {
  return invoke("export_oscal_document", { uuid, fileName });
}

export function deleteOscalDocument(uuid: string): Promise<void> {
  return invoke("delete_oscal_document", { uuid });
}
