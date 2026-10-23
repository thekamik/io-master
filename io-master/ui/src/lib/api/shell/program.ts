// The element "program" of the view Shell: its command and its data
// (src/views/shell/program/).
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

/** The state of the program. */
export function get(): Promise<ProgramView> {
  return invoke("program_get");
}
