// The element "user menu" of the view Shell: its command
// (src/views/shell/user_menu/).

import { invoke } from "@tauri-apps/api/core";

/** Logs out. */
export function logOut(): Promise<void> {
  return invoke("user_menu_log_out");
}
