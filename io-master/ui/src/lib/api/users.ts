// The commands of the view Users, one function per command of its entry in
// the program (src/views/users/users.rs), named as the command:
// `loginFormLogIn` calls `login_form_log_in`: the account of the evaluator,
// its session and the encryption of its database. None gives data back: the
// page then reads the state of the program again (shell.programGet).

import { invoke } from "@tauri-apps/api/core";

// ── Before anybody is logged in ──────────────────────────────────────

/** Makes the installation and logs its user in. Called by SetupForm.svelte, "Install". */
export function setupFormInstall(fields: {
  name: string;
  password: string;
  passwordRepeat: string;
}): Promise<void> {
  return invoke("setup_form_install", fields);
}

/** Logs in. Called by LoginForm.svelte, "Log in". */
export function loginFormLogIn(name: string, password: string): Promise<void> {
  return invoke("login_form_log_in", { name, password });
}

// ── The account of the logged-in evaluator ───────────────────────────

/** Changes the password of the logged-in evaluator. Called by PasswordForm.svelte of the page User. */
export function passwordFormChange(
  current: string,
  password: string,
  passwordRepeat: string,
): Promise<void> {
  return invoke("password_form_change", { current, password, passwordRepeat });
}

// ── The end of the session ───────────────────────────────────────────

/** Logs out. Called by UserMenu.svelte, "Log out". */
export function userMenuLogOut(): Promise<void> {
  return invoke("user_menu_log_out");
}

/** Logs out and closes the program. Called by UserMenu.svelte, "Log out and close". */
export function userMenuClose(): Promise<void> {
  return invoke("user_menu_close");
}
