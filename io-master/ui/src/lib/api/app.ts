// The password of the logged-in evaluator (tauri_ui/commands/account.rs);
// moved to the view Settings with that view.

import { invoke } from "@tauri-apps/api/core";

export function changePassword(current: string, password: string, passwordRepeat: string): Promise<void> {
  return invoke("change_password", { current, password, passwordRepeat });
}
