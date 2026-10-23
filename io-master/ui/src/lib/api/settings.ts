// The commands of the view Settings, one function per command of its entry
// in the program (src/views/settings/settings.rs), named as the command:
// `apiKeyGet` calls `api_key_get`. The API key itself never comes back.
//
// The types mirror the view.rs of the elements (src/views/settings/); a
// change there needs the same change here.

import { invoke } from "@tauri-apps/api/core";

// connection_list/view.rs

export interface ConnectionListView {
  /** The connections, by name. */
  connections: { id: number; name: string }[];
  /** The connection the chat uses. */
  active: number | null;
}

// connection_form/view.rs

/** The fields of a connection, as the page edits and sends them. */
export interface ConnectionFields {
  name: string;
  url: string;
  model: string;
  ca_certificate: string;
  headers: string;
  body: string;
  /** The JSON of one message in {{messages}}, with {{role}} and {{text}}. */
  message_template: string;
  user_role: string;
  model_role: string;
  system_role: string;
  system_prompt: string;
  answer: "stream" | "whole";
  text_path: string;
  stop_path: string;
  cut_reasons: string;
  refused_reasons: string;
  error_path: string;
  model_path: string;
  input_tokens_path: string;
  output_tokens_path: string;
  done_marker: string;
}

/** A connection as the program shows it; never with the key. */
export interface ConnectionView extends ConnectionFields {
  /** null: the template of a new connection, or a preset. */
  id: number | null;
}

/** Ready fields of a well-known service; choosing one only fills the form. */
export interface PresetView {
  id: string;
  title: string;
  description: string;
  fields: ConnectionView;
}

export interface NewConnectionView {
  template: ConnectionView;
  presets: PresetView[];
}

export interface SavedView {
  id: number;
  notice: string;
}

// api_key/view.rs

export interface ApiKeyView {
  has_key: boolean;
  /** A key is stored, but no header uses {{api_key}}. */
  key_unused: boolean;
  header_uses_key: boolean;
}

// connection_check/view.rs

/** The report of "Check connection"; the key masked everywhere. */
export interface CheckView {
  works: boolean;
  outcome: string;
  url: string;
  headers: { name: string; value: string }[];
  body: string;
  status: number | null;
  raw: string;
  text: string;
  model: string | null;
  input_tokens: number;
  output_tokens: number;
}

// ── When the tab AI opens ────────────────────────────────────────────

/** The connections, and the one the chat uses. Called by ConnectionList.svelte when the tab AI opens and after a change. */
export function connectionListGet(): Promise<ConnectionListView> {
  return invoke("connection_list_get");
}

/** The template of a new connection and the presets. Called by ConnectionForm.svelte when the tab AI opens. */
export function connectionFormNew(): Promise<NewConnectionView> {
  return invoke("connection_form_new");
}

// ── The actions of the elements ──────────────────────────────────────

/** A stored connection. Called by ConnectionForm.svelte when a connection is chosen. */
export function connectionFormGet(id: number): Promise<ConnectionView> {
  return invoke("connection_form_get", { id });
}

/** Saves a connection; `id` null: a new one. Called by ConnectionForm.svelte, "Save". */
export function connectionFormSave(
  id: number | null,
  connection: ConnectionFields,
): Promise<SavedView> {
  return invoke("connection_form_save", { id, connection });
}

/** Deletes a connection with its key. Called by ConnectionForm.svelte, "Delete". */
export function connectionFormDelete(id: number): Promise<void> {
  return invoke("connection_form_delete", { id });
}

/** Makes a connection the one the chat uses. Called by ConnectionForm.svelte. */
export function connectionFormActivate(id: number): Promise<void> {
  return invoke("connection_form_activate", { id });
}

/** The instruction for a chatbot that helps to fill one field. Called by ConnectionForm.svelte. */
export function connectionFormFieldHelp(
  field: string,
  connection: ConnectionFields,
): Promise<string> {
  return invoke("connection_form_field_help", { field, connection });
}

/** Whether a key is stored, never the key. Called by ApiKey.svelte when a connection is chosen. */
export function apiKeyGet(id: number): Promise<ApiKeyView> {
  return invoke("api_key_get", { id });
}

/** Stores the key of a connection. Called by ApiKey.svelte, "Store". */
export function apiKeyStore(id: number, apiKey: string): Promise<ApiKeyView> {
  return invoke("api_key_store", { id, apiKey });
}

/** Deletes the key of a connection. Called by ApiKey.svelte, "Delete". */
export function apiKeyDelete(id: number): Promise<ApiKeyView> {
  return invoke("api_key_delete", { id });
}

/** Sends one short message over a stored connection. Called by ConnectionCheck.svelte, "Check". */
export function connectionCheckRun(id: number): Promise<CheckView> {
  return invoke("connection_check_run", { id });
}
