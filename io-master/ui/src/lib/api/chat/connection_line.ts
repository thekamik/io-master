// The element "connection line" of the view Chat: its command and its
// data (src/views/chat/connection_line/).
//
// The types mirror src/views/chat/connection_line/view.rs.

import { invoke } from "@tauri-apps/api/core";

/** The connection the chat uses. */
export interface ConnectionLineView {
  name: string;
  /** Who writes the replies: the model, or the name of the connection. */
  writer: string;
}

/** The connection the chat uses now; null when none is chosen. */
export function get(): Promise<ConnectionLineView | null> {
  return invoke("connection_line_get");
}
