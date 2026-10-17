// The tab AI of Settings (serializer/ai_config_view.rs; tauri_ui/commands/ai.rs).
//
// The types mirror the views of src/tauri_ui/serializer/; a change
// there needs the same change here.

import { invoke } from "@tauri-apps/api/core";

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
  /** null: the template of a new connection. */
  id: number | null;
  has_key: boolean;
  /** A key is stored, but no header uses {{api_key}}. */
  key_unused: boolean;
}

/** Ready fields of a well-known service; choosing one only fills the form. */
export interface PresetView {
  id: string;
  title: string;
  description: string;
  fields: ConnectionView;
}

export interface AiConfigView {
  connections: ConnectionView[];
  /** The connection the chat uses. */
  active: number | null;
  template: ConnectionView;
  presets: PresetView[];
}

export interface SavedView {
  config: AiConfigView;
  id: number;
  notice: string;
}

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

export function aiConfig(): Promise<AiConfigView> {
  return invoke("ai_config");
}

/** Saves a connection; `id` null: a new one. */
export function saveAiConnection(id: number | null, connection: ConnectionFields): Promise<SavedView> {
  return invoke("save_ai_connection", { id, connection });
}

export function deleteAiConnection(id: number): Promise<AiConfigView> {
  return invoke("delete_ai_connection", { id });
}

/** Makes a connection the one the chat uses. */
export function activateAiConnection(id: number): Promise<AiConfigView> {
  return invoke("activate_ai_connection", { id });
}

export function storeAiKey(id: number, apiKey: string): Promise<AiConfigView> {
  return invoke("store_ai_key", { id, apiKey });
}

export function deleteAiKey(id: number): Promise<AiConfigView> {
  return invoke("delete_ai_key", { id });
}

/** The instruction for a chatbot that helps to fill one field of the form. */
export function aiFieldInstruction(field: string, connection: ConnectionFields): Promise<string> {
  return invoke("ai_field_instruction", { field, connection });
}

/** Sends one short message over a stored connection. */
export function checkAiConnection(id: number): Promise<CheckView> {
  return invoke("check_ai_connection", { id });
}
