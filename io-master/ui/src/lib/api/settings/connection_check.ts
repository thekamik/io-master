// The element "check of a connection" of the view Settings: its command
// and its data (src/views/settings/connection_check/).
//
// The types mirror src/views/settings/connection_check/view.rs.

import { invoke } from "@tauri-apps/api/core";

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

/** Sends one short message over a stored connection. */
export function run(id: number): Promise<CheckView> {
  return invoke("connection_check_run", { id });
}
