// The element "scanner settings" of the view Scan: its commands and its
// data (src/views/scan/scanner_settings/).
//
// The types mirror src/views/scan/scanner_settings/view.rs; a change
// there needs the same change here.

import { invoke } from "@tauri-apps/api/core";

import type { ColumnView } from "./ports_table";

/** The settings of the scanner, and what can be chosen. */
export interface ScannerSettingsView {
  all_types: boolean;
  /** The chosen kinds (base names) when not all. */
  types: string[];
  /** The keys of the chosen columns, in the order of the table. */
  columns: string[];
  /** The settings in one line: "ttyACM · 5 columns". */
  summary: string;
  choices: {
    types: string[];
    columns: ColumnView[];
  };
}

/** The stored settings. */
export function get(): Promise<ScannerSettingsView> {
  return invoke("scanner_settings_get");
}

/** Stores the settings; gives them back as stored. */
export function save(allTypes: boolean, types: string[], columns: string[]): Promise<ScannerSettingsView> {
  return invoke("scanner_settings_save", { allTypes, types, columns });
}
