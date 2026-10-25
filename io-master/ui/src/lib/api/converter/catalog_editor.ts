// The commands of the editor of a catalog, a screen of the view Converter,
// one function per command of its entry in the program
// (src/views/converter/catalog_editor/catalog_editor.rs), named as the
// command: `proposalsGet` calls `proposals_get`. The tree of the catalog is a
// component (components/catalog_tree.ts).
//
// The types mirror the view.rs of the elements
// (src/views/converter/catalog_editor/); a change there needs the same
// change here.

import { Channel, invoke } from "@tauri-apps/api/core";

import type { PartRowView } from "../components/part_rows";

// proposals/view.rs
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

// ai_builder/view.rs
export interface SourceView {
  id: number;
  title: string;
  page_count: number;
}

export interface PageRange {
  document_id: number;
  first_page: number;
  last_page: number;
}

export type BuildEvent =
  | {
      event: "part";
      index: number;
      count: number;
      document: string;
      first_page: number;
      last_page: number;
    }
  | { event: "writing"; characters: number }
  | { event: "proposed"; added: number; dropped: string[] }
  | { event: "failed"; message: string }
  | { event: "end"; stopped: boolean };

// ai_instructions/view.rs
/** The instructions of the two tasks of the catalogs. */
export type Which = "catalog_contents" | "catalog_outline" | "catalog_section";

// conversion_wizard/view.rs
/** Pages of a document. */
export interface PagesView {
  document_id: number;
  first_page: number;
  last_page: number;
}

export type Stage = "contents" | "groups" | "controls" | "review" | "finished";
export type GroupState = "todo" | "proposed" | "done";

/** A group whose controls the AI reads. */
export interface WizardGroupView {
  id: string;
  title: string;
  pages: PagesView | null;
  state: GroupState;
  controls: number;
}

/** The wizard of a conversion. */
export interface WizardView {
  stage: Stage;
  contents: PagesView | null;
  groups: WizardGroupView[];
  review: { proposals: number; quotes_not_found: number; groups_without_controls: number };
  /** The agent with knowledge of the works of the AI of the catalog (step 57.6); null: none. */
  agent: number | null;
}

// object_form/view.rs
export interface ObjectView {
  kind: "group" | "control";
  id: string;
  title: string;
  parts: PartRowView[];
  /** Its own fields as JSON of OSCAL (without what it holds). */
  fields: Record<string, unknown>;
}

// new_object/view.rs
/** Where a new object goes. */
export type Place =
  | { kind: "top" }
  | { kind: "group"; id: string }
  | { kind: "control"; id: string };

// object_quotes/view.rs
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

// ── When the editor opens ────────────────────────────────────────────

/** Called by Proposals.svelte, when the editor opens and after every change. */
export function proposalsGet(uuid: string): Promise<ProposalView[]> {
  return invoke("proposals_get", { uuid });
}

/** Called by AiBuilder.svelte, when the editor opens. */
export function aiBuilderSources(): Promise<SourceView[]> {
  return invoke("ai_builder_sources");
}

/** Called by AiInstructions.svelte, when its dialog opens. */
export function aiInstructionsGet(which: Which): Promise<string> {
  return invoke("ai_instructions_get", { which });
}

// ── The actions of the elements ──────────────────────────────────────

/** Accepts a proposal, as it is (`corrected` null) or corrected. Called by Proposals.svelte, "Accept". */
export function proposalsAccept(
  id: number,
  corrected: unknown | null,
): Promise<void> {
  return invoke("proposals_accept", { id, corrected });
}

/** Called by Proposals.svelte, "Reject". */
export function proposalsReject(id: number): Promise<void> {
  return invoke("proposals_reject", { id });
}

/** The AI reads a table of contents and proposes the groups. Called by AiBuilder.svelte, "Propose the groups". */
export function aiBuilderGroups(
  uuid: string,
  range: PageRange,
  onEvent: (event: BuildEvent) => void,
): Promise<void> {
  const channel = new Channel<BuildEvent>();
  channel.onmessage = onEvent;

  return invoke("ai_builder_groups", { uuid, range, onEvent: channel });
}

/** The AI reads pages of a group (null: the top) and proposes its controls. Called by AiBuilder.svelte, "Propose the controls". */
export function aiBuilderControls(
  uuid: string,
  group: string | null,
  ranges: PageRange[],
  partSize: number,
  onEvent: (event: BuildEvent) => void,
): Promise<void> {
  const channel = new Channel<BuildEvent>();
  channel.onmessage = onEvent;

  return invoke("ai_builder_controls", {
    uuid,
    group,
    ranges,
    partSize,
    onEvent: channel,
  });
}

/** Called by ConversionWizard.svelte, when the editor opens and after a change. */
export function conversionWizardGet(uuid: string): Promise<WizardView> {
  return invoke("conversion_wizard_get", { uuid });
}

/** Called by ConversionWizard.svelte, the agent with knowledge of the catalog. */
export function conversionWizardAgent(uuid: string, agent: number | null): Promise<WizardView> {
  return invoke("conversion_wizard_agent", { uuid, agent });
}

/** The AI finds the pages of the table of contents. Called by ConversionWizard.svelte, "Find it with the AI". */
export function conversionWizardFindContents(
  uuid: string,
  documentId: number,
  onEvent: (event: BuildEvent) => void,
): Promise<WizardView> {
  const channel = new Channel<BuildEvent>();
  channel.onmessage = onEvent;

  return invoke("conversion_wizard_find_contents", { uuid, documentId, onEvent: channel });
}

/** Called by ConversionWizard.svelte, "Keep these pages". */
export function conversionWizardContents(uuid: string, pages: PagesView): Promise<WizardView> {
  return invoke("conversion_wizard_contents", { uuid, pages });
}

/** Called by ConversionWizard.svelte, "Next", "Back", "Finish". */
export function conversionWizardStage(uuid: string, stage: Stage): Promise<WizardView> {
  return invoke("conversion_wizard_stage", { uuid, stage });
}

/** Called by ConversionWizard.svelte, the pages of a group. */
export function conversionWizardGroupPages(uuid: string, group: string, pages: PagesView): Promise<WizardView> {
  return invoke("conversion_wizard_group_pages", { uuid, group, pages });
}

/** Called by ConversionWizard.svelte, after the AI read a group and "This group is done". */
export function conversionWizardGroupState(uuid: string, group: string, groupState: GroupState): Promise<WizardView> {
  return invoke("conversion_wizard_group_state", { uuid, group, groupState });
}

/** Called by AiBuilder.svelte and ConversionWizard.svelte, "Stop". */
export function aiBuilderStop(): Promise<void> {
  return invoke("ai_builder_stop");
}

/** Stores the instructions; an empty text brings back the default ones. Called by AiInstructions.svelte, "Save". */
export function aiInstructionsSave(
  which: Which,
  text: string,
): Promise<string> {
  return invoke("ai_instructions_save", { which, text });
}

/** Called by ObjectForm.svelte, when an object is chosen. */
export function objectFormGet(uuid: string, id: string): Promise<ObjectView> {
  return invoke("object_form_get", { uuid, id });
}

/** Called by ObjectForm.svelte, "Save" of a group. */
export function objectFormSaveGroup(
  uuid: string,
  id: string,
  group: unknown,
): Promise<void> {
  return invoke("object_form_save_group", { uuid, id, group });
}

/** Called by ObjectForm.svelte, "Save" of a control. */
export function objectFormSaveControl(
  uuid: string,
  id: string,
  control: unknown,
): Promise<void> {
  return invoke("object_form_save_control", { uuid, id, control });
}

/** Called by ObjectForm.svelte, "Delete". */
export function objectFormDelete(uuid: string, id: string): Promise<void> {
  return invoke("object_form_delete", { uuid, id });
}

/** Called by NewObject.svelte, "Add" of a group. */
export function newObjectAddGroup(
  uuid: string,
  place: Place,
  group: unknown,
): Promise<void> {
  return invoke("new_object_add_group", { uuid, place, group });
}

/** Called by NewObject.svelte, "Add" of a control. */
export function newObjectAddControl(
  uuid: string,
  place: Place,
  control: unknown,
): Promise<void> {
  return invoke("new_object_add_control", { uuid, place, control });
}

/** Called by ObjectQuotes.svelte, when an object is chosen and after a change. */
export function objectQuotesGet(
  uuid: string,
  objectId: string,
): Promise<ObjectQuotesView> {
  return invoke("object_quotes_get", { uuid, objectId });
}

/** Called by ObjectQuotes.svelte, a quote taken in the viewer. */
export function objectQuotesAdd(
  uuid: string,
  objectId: string,
  source: number,
  page: number,
  text: string,
): Promise<void> {
  return invoke("object_quotes_add", { uuid, objectId, source, page, text });
}

/** Called by ObjectQuotes.svelte, "Delete". */
export function objectQuotesDelete(id: number): Promise<void> {
  return invoke("object_quotes_delete", { id });
}
