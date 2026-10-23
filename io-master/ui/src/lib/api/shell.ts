// The commands of the view Shell, one function per command of its entry in
// the program (src/views/shell/shell.rs), named as the command:
// `programGet` calls `program_get`.
//
// The types mirror src/views/shell/program/view.rs; a change there needs
// the same change here.

import { invoke } from "@tauri-apps/api/core";

export interface PageView {
  id: string;
  title: string;
}

export interface WarningView {
  level: "danger" | "warning";
  text: string;
}

/** The state of the program. */
export interface ProgramView {
  installed: boolean;
  role: "workstation" | "master" | null;
  user: string | null;
  pages: PageView[];
  warnings: WarningView[];
  /** The version of the program (`Cargo.toml`): "0.1.0". */
  version: string;
  /** The folder of the installation. */
  data_dir: string;
}

// ── When the view opens ──────────────────────────────────────────────

/**
 * The state of the program. Called by App.svelte when the window opens and after every change of the
 * state (the setup, the login, the logout).
 */
export function programGet(): Promise<ProgramView> {
  return invoke("program_get");
}

// ── The actions of the elements ──────────────────────────────────────

/** Logs out. Called by UserMenu.svelte, "Log out". */
export function userMenuLogOut(): Promise<void> {
  return invoke("user_menu_log_out");
}
