// The commands of the view Devices, one function per command of its entry
// in the program (src/views/devices/devices.rs), named as the command:
// `portsTableScan` calls `ports_table_scan`.
//
// The types mirror the view.rs of the elements (src/views/devices/); a
// change there needs the same change here.

import { invoke } from "@tauri-apps/api/core";

/** A column of the table (src/views/devices/column_view.rs). */
export interface ColumnView {
  key: string;
  title: string;
}

/** The table of ports (src/views/devices/ports_table/view.rs). */
export interface PortsTableView {
  columns: ColumnView[];
  /** One row per port, one text per column; empty when the scan failed. */
  rows: string[][];
  /** Why the ports could not be scanned; null when the scan worked. */
  problem: string | null;
}

/** The settings of the scanner, and what can be chosen (src/views/devices/scanner_settings/view.rs). */
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

// ── When the view opens ──────────────────────────────────────────────

/** The stored settings. Called by ScannerSettings.svelte when the view opens. */
export function scannerSettingsGet(): Promise<ScannerSettingsView> {
  return invoke("scanner_settings_get");
}

/**
 * Scans the serial ports with the stored settings. Called by PortsTable.svelte when the view opens,
 * on "Scan again", and after the settings are saved.
 */
export function portsTableScan(): Promise<PortsTableView> {
  return invoke("ports_table_scan");
}

// ── The actions of the elements ──────────────────────────────────────

/** Stores the settings; gives them back as stored. Called by ScannerSettings.svelte, "Save". */
export function scannerSettingsSave(allTypes: boolean, types: string[], columns: string[]): Promise<ScannerSettingsView> {
  return invoke("scanner_settings_save", { allTypes, types, columns });
}
