// The commands of the view Start, one function per command of its entry in
// the program (src/views/start/start.rs), named as the command:
// `loginFormLogIn` calls `login_form_log_in`. Neither gives data back: the
// page then reads the state of the program again (shell.programGet).

import { invoke } from "@tauri-apps/api/core";

// ── The actions of the elements ──────────────────────────────────────

/** Makes the installation and logs its user in. Called by SetupForm.svelte, "Install". */
export function setupFormInstall(fields: {
  name: string;
  password: string;
  passwordRepeat: string;
  role: string;
  roleConfirmed: boolean;
}): Promise<void> {
  return invoke("setup_form_install", fields);
}

/** Logs in. Called by LoginForm.svelte, "Log in". */
export function loginFormLogIn(name: string, password: string): Promise<void> {
  return invoke("login_form_log_in", { name, password });
}
