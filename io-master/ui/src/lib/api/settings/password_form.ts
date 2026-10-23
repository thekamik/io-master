// The element "password form" of the view Settings: its command
// (src/views/settings/password_form/).

import { invoke } from "@tauri-apps/api/core";

/** Changes the password of the logged-in evaluator. */
export function change(current: string, password: string, passwordRepeat: string): Promise<void> {
  return invoke("password_form_change", { current, password, passwordRepeat });
}
