// The commands of the settings of the AI (src/panels/views/settings/), one
// function per command, named as the command: `integrationList` calls
// `integration_list`. The types mirror the views of those files; a change
// there needs the same change here.

import { invoke } from "@tauri-apps/api/core";

/** The way the program speaks with an AI service. */
export type ProtocolName = "openai_chat" | "anthropic_messages";

/** How an answer of the AI ended. */
export type EndingName = "complete" | "cut" | "stopped" | "refused" | "failed";

/** A task of the AI. */
export type TaskName = "assessment" | "consultation";

/** An integration in the list (integrations.rs, IntegrationView). */
export interface IntegrationView {
  id: number;
  name: string;
  url: string;
  model: string;
  protocol: ProtocolName;
  caCertificate: string;
  contextSize: number;
  hasKey: boolean;
  active: boolean;
}

/** The fields of an integration as the page sends them; `id` 0 for a new
 * one (integrations.rs, IntegrationFields). */
export interface IntegrationFields {
  id: number;
  name: string;
  url: string;
  model: string;
  protocol: ProtocolName;
  caCertificate: string;
  contextSize: number;
}

/** What the check of an integration found (integrations.rs, CheckView). */
export interface CheckView {
  ending: EndingName;
  text: string;
  model: string;
  error: string | null;
  inputTokens: number;
  outputTokens: number;
  journal: number;
}

/** The instructions of a task (instructions.rs, InstructionsView). */
export interface InstructionsView {
  task: TaskName;
  text: string;
}

/** An entry of the journal in its list (journal.rs, JournalRowView). */
export interface JournalRowView {
  id: number;
  time: number;
  integration: string;
  model: string;
  agent: string;
  workUnit: number | null;
  ending: EndingName;
  inputTokens: number;
  outputTokens: number;
}

/** An entry of the journal whole (journal.rs, JournalEntryView). */
export interface JournalEntryView {
  row: JournalRowView;
  request: string;
  answer: string;
}

/** Every integration, the active one marked. */
export function integrationList(): Promise<IntegrationView[]> {
  return invoke("integration_list");
}

/** Stores a new integration; gives its id. */
export function integrationCreate(fields: IntegrationFields): Promise<number> {
  return invoke("integration_create", { fields });
}

/** Saves the data of an integration. */
export function integrationSave(fields: IntegrationFields): Promise<void> {
  return invoke("integration_save", { fields });
}

/** Makes an integration the active one. */
export function integrationActivate(id: number): Promise<void> {
  return invoke("integration_activate", { id });
}

/** Stores the API key of an integration; it is never given back. */
export function integrationSetKey(id: number, key: string): Promise<void> {
  return invoke("integration_set_key", { id, key });
}

/** Checks an integration with one short question. */
export function integrationCheck(id: number): Promise<CheckView> {
  return invoke("integration_check", { id });
}

/** The instructions of every task. */
export function instructionsGet(): Promise<InstructionsView[]> {
  return invoke("instructions_get");
}

/** Saves the instructions of a task. */
export function instructionsSave(task: TaskName, text: string): Promise<void> {
  return invoke("instructions_save", { task, text });
}

/** One page of the journal, older than `before` (the newest with null). */
export function journalList(before: number | null): Promise<JournalRowView[]> {
  return invoke("journal_list", { before });
}

/** An entry of the journal whole. */
export function journalGet(id: number): Promise<JournalEntryView> {
  return invoke("journal_get", { id });
}
