// The commands of the panel Account (src/panels/views/account/), one
// function per command, named as the command: `accountChangePassword` calls
// `account_change_password`. Each gives the new recovery code, shown once.

import { invoke } from "@tauri-apps/api/core";

import type { RecoveryCodeView } from "./program_start";

/** Changes the password; gives the new recovery code. Called by PasswordForm.svelte. */
export function accountChangePassword(fields: {
  current: string;
  password: string;
  passwordRepeated: string;
}): Promise<RecoveryCodeView> {
  return invoke("account_change_password", fields);
}

/** Makes a new recovery code with the password. Called by RecoveryCodeForm.svelte. */
export function accountNewRecoveryCode(password: string): Promise<RecoveryCodeView> {
  return invoke("account_new_recovery_code", { password });
}
