// The element "connection form" of the view Settings: its commands and its
// data (src/views/settings/connection_form/).
//
// The types mirror src/views/settings/connection_form/view.rs.

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

/** The template of a new connection and the presets. */
export function fresh(): Promise<NewConnectionView> {
  return invoke("connection_form_new");
}

/** A stored connection. */
export function get(id: number): Promise<ConnectionView> {
  return invoke("connection_form_get", { id });
}

/** Saves a connection; `id` null: a new one. */
export function save(id: number | null, connection: ConnectionFields): Promise<SavedView> {
  return invoke("connection_form_save", { id, connection });
}

/** Deletes a connection with its key. */
export function remove(id: number): Promise<void> {
  return invoke("connection_form_delete", { id });
}

/** Makes a connection the one the chat uses. */
export function activate(id: number): Promise<void> {
  return invoke("connection_form_activate", { id });
}

/** The instruction for a chatbot that helps to fill one field of the form. */
export function fieldHelp(field: string, connection: ConnectionFields): Promise<string> {
  return invoke("connection_form_field_help", { field, connection });
}
