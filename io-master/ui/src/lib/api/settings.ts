// The commands of the settings of the AI (src/panels/views/settings/), one
// function per command, named as the command: `integrationList` calls
// `integration_list`. The types mirror the views of those files; a change
// there needs the same change here.

import { invoke } from "@tauri-apps/api/core";

/** The way the program speaks with an AI service. */
export type ProtocolName = "openai_chat" | "anthropic_messages" | "custom";

/** How the answer of a custom service comes: one JSON document, or a stream of events. */
export type ResponseKindName = "whole" | "events";

/** How an answer of the AI ended (also the entries of a journal). */
export type EndingName = "complete" | "cut" | "stopped" | "refused" | "failed";

/** An integration in the list (integrations.rs, IntegrationView). */
export interface IntegrationView {
  id: number;
  name: string;
  url: string;
  model: string;
  protocol: ProtocolName;
  /** The custom format it is spoken with; given for the protocol "custom" alone. */
  customFormat: number | null;
  caCertificate: string;
  contextSize: number;
  /** The characters of a token, by which a question is measured. */
  charactersPerToken: number;
  connectTimeoutSeconds: number;
  streamSilenceSeconds: number;
  /** Why the context is too small for an ST of 100 pages; null when it is not. */
  contextWarning: string | null;
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
  /** The custom format it is spoken with; given for the protocol "custom" alone. */
  customFormat: number | null;
  caCertificate: string;
  contextSize: number;
  charactersPerToken: number;
  connectTimeoutSeconds: number;
  streamSilenceSeconds: number;
}

/** Where a new integration starts and the limits of the fields, as the
 * program gives them (integrations.rs, IntegrationFormView). */
export interface IntegrationFormView {
  name: string;
  protocol: ProtocolName;
  url: string;
  model: string;
  contextSize: number;
  charactersPerToken: number;
  connectTimeoutSeconds: number;
  streamSilenceSeconds: number;
  minContext: number;
  maxCharactersPerToken: number;
  maxConnectTimeoutSeconds: number;
  maxStreamSilenceSeconds: number;
}

/** What the check of an integration found (integrations.rs, CheckView). */
export interface CheckView {
  ending: EndingName;
  text: string;
  model: string;
  error: string | null;
  inputTokens: number;
  outputTokens: number;
  /** Why the context of the integration is too small for an ST of 100 pages; null when it is not. */
  contextWarning: string | null;
}

/** A custom format, both its JSON texts whole; `id` 0 for one not stored
 * yet (custom_formats.rs, CustomFormatView). */
export interface CustomFormatView {
  id: number;
  name: string;
  /** The JSON of a question, with the placeholders of a request. */
  request: string;
  /** The JSON of an answer, with the placeholders of a response. */
  response: string;
  responseKind: ResponseKindName;
  keyHeader: string;
  keyPrefix: string;
}

/** What a trial of a format gives (custom_formats.rs, TrialView). */
export interface TrialView {
  /** The request as it would be sent, filled with a sample of a question. */
  request: string;
  /** Whether a sample of an answer was given and read. */
  read: boolean;
  text: string;
  model: string;
  inputTokens: number;
  outputTokens: number;
}

/** Every custom format, by its name. */
export function customFormatList(): Promise<CustomFormatView[]> {
  return invoke("custom_format_list");
}

/** Where a new custom format starts. */
export function customFormatNew(): Promise<CustomFormatView> {
  return invoke("custom_format_new");
}

/** Stores a new custom format; gives its id. */
export function customFormatCreate(fields: CustomFormatView): Promise<number> {
  return invoke("custom_format_create", { fields });
}

/** Writes a stored custom format anew. */
export function customFormatSave(fields: CustomFormatView): Promise<void> {
  return invoke("custom_format_save", { fields });
}

/** Tries a format as typed, without storing it and without any connection. */
export function customFormatTry(fields: CustomFormatView, sample: string): Promise<TrialView> {
  return invoke("custom_format_try", { fields, sample });
}

/** Every integration, the active one marked. */
export function integrationList(): Promise<IntegrationView[]> {
  return invoke("integration_list");
}

/** Where a new integration of a protocol starts (of the program's first
 * protocol, without one) and the limits of its fields. */
export function integrationForm(protocol: ProtocolName | null): Promise<IntegrationFormView> {
  return invoke("integration_form", { protocol });
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
