// The element "instructions to the AI" of the editor of a catalog: its
// commands (src/views/evaluation_plan/catalog_editor/ai_instructions/).

import { invoke } from "@tauri-apps/api/core";

/** The instructions of the two tasks of the catalogs. */
export type Which = "catalog_outline" | "catalog_section";

export function get(which: Which): Promise<string> {
  return invoke("ai_instructions_get", { which });
}

/** Stores the instructions; an empty text brings back the default ones. */
export function save(which: Which, text: string): Promise<string> {
  return invoke("ai_instructions_save", { which, text });
}
