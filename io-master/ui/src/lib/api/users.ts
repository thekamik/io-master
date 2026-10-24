// The commands of the view Users, one function per command of its entry in
// the program (src/views/users/users.rs), named as the command:
// `loginFormLogIn` calls `login_form_log_in`: the account of the evaluator,
// its session and the encryption of its database. Those that make a recovery
// code give it back, to be shown once; the page then reads the state of the
// program again (shell.programGet).

import { invoke } from "@tauri-apps/api/core";
import type { RecoveryCodeView } from "./components/recovery_code";

// ── Before anybody is logged in ──────────────────────────────────────

/** Makes the installation, logs its user in, gives the first recovery code. Called by SetupForm.svelte, "Install". */
export function setupFormInstall(fields: {
  name: string;
  password: string;
  passwordRepeat: string;
}): Promise<RecoveryCodeView> {
  return invoke("setup_form_install", fields);
}

/** Logs in. Called by LoginForm.svelte, "Log in". */
export function loginFormLogIn(name: string, password: string): Promise<void> {
  return invoke("login_form_log_in", { name, password });
}

/**
 * Sets a new password with the recovery code and logs in; gives the new code.
 * Called by LoginForm.svelte, "I do not remember my password".
 */
export function loginFormRecover(fields: {
  name: string;
  code: string;
  password: string;
  passwordRepeat: string;
}): Promise<RecoveryCodeView> {
  return invoke("login_form_recover", fields);
}

// ── The account of the logged-in evaluator ───────────────────────────

/** Changes the password; gives the new recovery code. Called by PasswordForm.svelte of the page User. */
export function passwordFormChange(
  current: string,
  password: string,
  passwordRepeat: string,
): Promise<RecoveryCodeView> {
  return invoke("password_form_change", { current, password, passwordRepeat });
}

/** Makes a new recovery code; needs the password. Called by RecoveryCodeForm.svelte of the page User. */
export function recoveryCodeFormReplace(password: string): Promise<RecoveryCodeView> {
  return invoke("recovery_code_form_replace", { password });
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
