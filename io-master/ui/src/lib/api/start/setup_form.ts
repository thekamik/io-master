// The element "setup form" of the view Start: its command
// (src/views/start/setup_form/).

import { invoke } from "@tauri-apps/api/core";

/** Makes the installation and logs its user in. */
export function install(fields: {
  name: string;
  password: string;
  passwordRepeat: string;
  role: string;
  roleConfirmed: boolean;
}): Promise<void> {
  return invoke("setup_form_install", fields);
}
