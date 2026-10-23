// The element "list of connections" of the view Settings: its command and
// its data (src/views/settings/connection_list/).
//
// The types mirror src/views/settings/connection_list/view.rs.

import { invoke } from "@tauri-apps/api/core";

export interface ConnectionListView {
  /** The connections, by name. */
  connections: { id: number; name: string }[];
  /** The connection the chat uses. */
  active: number | null;
}

/** The connections, and the one the chat uses. */
export function get(): Promise<ConnectionListView> {
  return invoke("connection_list_get");
}
