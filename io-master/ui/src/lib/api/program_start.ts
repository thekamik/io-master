// The commands of the first start and the login (src/panels/views/program_start/),
// one function per command, named as the command: `loginLogIn` calls
// `login_log_in`. The types mirror the views of those files; a change there
// needs the same change here.

import { invoke } from "@tauri-apps/api/core";

/** What the window shows at its start (login.rs, ProgramView). */
export interface ProgramView {
  /** Whether the program is installed: if not, the first start. */
  installed: boolean;
  /** The name of the user logged in; null: the login. */
  loggedIn: string | null;
}

/** The recovery code, shown once (src/panels/views/recovery_code.rs,
 * RecoveryCodeView); also given by the commands of account.ts. */
export interface RecoveryCodeView {
  /** Seven groups of five characters joined by "-". */
  code: string;
}

/** Makes the installation, logs its user in, gives the recovery code. Called by InstallationForm.svelte. */
export function installationCreate(fields: {
  name: string;
  password: string;
  passwordRepeated: string;
}): Promise<RecoveryCodeView> {
  return invoke("installation_create", fields);
}

/** The state of the program. Called by App.svelte at the start and after every change. */
export function loginGet(): Promise<ProgramView> {
  return invoke("login_get");
}

/** Logs in. Called by LoginForm.svelte. */
export function loginLogIn(name: string, password: string): Promise<void> {
  return invoke("login_log_in", { name, password });
}

/** Sets a new password with the recovery code, logs in, gives the new code.
 * Called by LoginForm.svelte. */
export function loginRecover(fields: {
  name: string;
  code: string;
  password: string;
  passwordRepeated: string;
}): Promise<RecoveryCodeView> {
  return invoke("login_recover", fields);
}

/** Logs out. Called by App.svelte. */
export function loginLogOut(): Promise<void> {
  return invoke("login_log_out");
}
