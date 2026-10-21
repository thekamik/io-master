// The element "login form" of the view Start: its command
// (src/views/start/login_form/).

import { invoke } from "@tauri-apps/api/core";

/** Logs in. */
export function logIn(name: string, password: string): Promise<void> {
  return invoke("login_form_log_in", { name, password });
}
