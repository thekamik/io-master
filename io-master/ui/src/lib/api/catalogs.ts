// The catalogs of OSCAL made in the program (step 41.3;
// serializer/catalog_view.rs, tauri_ui/commands/catalogs.rs): the
// catalog as its JSON of OSCAL, its quotes and the proposals of the AI
// (the evaluation support, docs/oscal.md, section 6).
//
// The types mirror the views of src/tauri_ui/serializer/; a change
// there needs the same change here.

import { Channel, invoke } from "@tauri-apps/api/core";

import type { OscalDocumentView } from "./plans/oscal_documents";

/** A part of a control or group (statement, item, guidance, ...). */
export interface OscalPart {
  id?: string;
  name: string;
  title?: string;
  prose?: string;
  parts?: OscalPart[];
  [key: string]: unknown;
}

/** A control of OSCAL, as far as the editor reads it. */
export interface OscalControl {
  id: string;
  title: string;
  parts?: OscalPart[];
  controls?: OscalControl[];
  [key: string]: unknown;
}

/** A group of OSCAL, as far as the editor reads it. */
export interface OscalGroup {
  id?: string;
  title: string;
  parts?: OscalPart[];
  groups?: OscalGroup[];
  controls?: OscalControl[];
  [key: string]: unknown;
}

export interface OscalCatalog {
  uuid: string;
  metadata: { title: string; version: string; [key: string]: unknown };
  groups?: OscalGroup[];
  controls?: OscalControl[];
  [key: string]: unknown;
}

export interface QuoteView {
  id: number;
  object_id: string;
  source_document_id: number;
  page: number;
  text: string;
  /** Whether the text was found on its page. */
  found: boolean;
}

export interface DraftView {
  source_document_id: number;
  page: number;
  text: string;
  found: boolean;
}

export interface ProposalView {
  id: number;
  kind: "group" | "control";
  parent_id: string | null;
  /** The group or the control, as its JSON of OSCAL. */
  content: OscalGroup | OscalControl;
  quotes: DraftView[];
  note: string;
  model: string;
}

export interface CatalogView {
  document: OscalDocumentView;
  catalog: OscalCatalog;
  quotes: QuoteView[];
  proposals: ProposalView[];
}

/** Where a new object goes. */
export type Place = { kind: "top" } | { kind: "group"; id: string } | { kind: "control"; id: string };

export interface PageRange {
  document_id: number;
  first_page: number;
  last_page: number;
}

export type BuildEvent =
  | { event: "part"; index: number; count: number; document: string; first_page: number; last_page: number }
  | { event: "writing"; characters: number }
  | { event: "proposed"; added: number; dropped: string[] }
  | { event: "failed"; message: string }
  | { event: "end"; stopped: boolean };

/** The instructions of the two tasks of the catalogs. */
export type CatalogTask = "catalog_outline" | "catalog_section";

/** Stops the work of the AI on the catalog. */
export function stopWork(): Promise<void> {
  return invoke("stop_work");
}

export function openCatalog(uuid: string): Promise<CatalogView> {
  return invoke("open_catalog", { uuid });
}

export function addGroup(uuid: string, place: Place, group: unknown): Promise<CatalogView> {
  return invoke("add_catalog_group", { uuid, place, group });
}

export function updateGroup(uuid: string, id: string, group: unknown): Promise<CatalogView> {
  return invoke("update_catalog_group", { uuid, id, group });
}

export function addControl(uuid: string, place: Place, control: unknown): Promise<CatalogView> {
  return invoke("add_catalog_control", { uuid, place, control });
}

export function updateControl(uuid: string, id: string, control: unknown): Promise<CatalogView> {
  return invoke("update_catalog_control", { uuid, id, control });
}

export function deleteObject(uuid: string, id: string): Promise<CatalogView> {
  return invoke("delete_catalog_object", { uuid, id });
}

export function addQuote(uuid: string, objectId: string, source: number, page: number, text: string): Promise<CatalogView> {
  return invoke("add_catalog_quote", { uuid, objectId, source, page, text });
}

export function deleteQuote(uuid: string, id: number): Promise<CatalogView> {
  return invoke("delete_catalog_quote", { uuid, id });
}

/** Accepts a proposal as it is, or as `corrected` (its JSON of OSCAL). */
export function acceptProposal(uuid: string, id: number, corrected: unknown | null): Promise<CatalogView> {
  return invoke("accept_catalog_proposal", { uuid, id, corrected });
}

export function rejectProposal(uuid: string, id: number): Promise<CatalogView> {
  return invoke("reject_catalog_proposal", { uuid, id });
}

export function catalogInstructions(which: CatalogTask): Promise<string> {
  return invoke("catalog_instructions", { which });
}

/** Stores the instructions; empty text brings back the default ones. */
export function saveCatalogInstructions(which: CatalogTask, text: string): Promise<string> {
  return invoke("save_catalog_instructions", { which, text });
}

/** The AI reads a table of contents and proposes the groups. Stopped by stopWork(). */
export function proposeGroups(uuid: string, range: PageRange, onEvent: (event: BuildEvent) => void): Promise<CatalogView> {
  const channel = new Channel<BuildEvent>();
  channel.onmessage = onEvent;

  return invoke("propose_catalog_groups", { uuid, range, onEvent: channel });
}

/** The AI reads pages of a group (null: the top) and proposes its controls. */
export function proposeControls(
  uuid: string,
  group: string | null,
  ranges: PageRange[],
  partSize: number,
  onEvent: (event: BuildEvent) => void,
): Promise<CatalogView> {
  const channel = new Channel<BuildEvent>();
  channel.onmessage = onEvent;

  return invoke("propose_catalog_controls", { uuid, group, ranges, partSize, onEvent: channel });
}
