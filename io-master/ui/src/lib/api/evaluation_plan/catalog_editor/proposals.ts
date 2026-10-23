// The element "proposals" of the editor of a catalog: its commands and its
// data (src/views/evaluation_plan/catalog_editor/proposals/).

import { invoke } from "@tauri-apps/api/core";

import type { PartRowView } from "../../components/part_rows";

export interface DraftView {
  source_document_id: number;
  document: string;
  page: number;
  text: string;
  found: boolean;
}

export interface ProposalView {
  id: number;
  kind: "group" | "control";
  parent_id: string | null;
  object_id: string;
  title: string;
  parts: PartRowView[];
  /** The object as its JSON of OSCAL, for a correction. */
  content: unknown;
  quotes: DraftView[];
  note: string;
  model: string;
}

export function get(uuid: string): Promise<ProposalView[]> {
  return invoke("proposals_get", { uuid });
}

/** Accepts a proposal, as it is (`corrected` null) or corrected. */
export function accept(id: number, corrected: unknown | null): Promise<void> {
  return invoke("proposals_accept", { id, corrected });
}

export function reject(id: number): Promise<void> {
  return invoke("proposals_reject", { id });
}
