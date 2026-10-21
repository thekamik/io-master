// The element "API key" of the view Settings: its commands and its data
// (src/views/settings/api_key/). The key itself never comes back.
//
// The types mirror src/views/settings/api_key/view.rs.

import { invoke } from "@tauri-apps/api/core";

export interface ApiKeyView {
  has_key: boolean;
  /** A key is stored, but no header uses {{api_key}}. */
  key_unused: boolean;
  header_uses_key: boolean;
}

export function get(id: number): Promise<ApiKeyView> {
  return invoke("api_key_get", { id });
}

export function store(id: number, apiKey: string): Promise<ApiKeyView> {
  return invoke("api_key_store", { id, apiKey });
}

export function remove(id: number): Promise<ApiKeyView> {
  return invoke("api_key_delete", { id });
}
