// The scan of the serial ports (serializer/scan_view.rs; tauri_ui/commands/scan.rs).
//
// The types mirror the views of src/tauri_ui/serializer/; a change
// there needs the same change here.

import { invoke } from "@tauri-apps/api/core";

export interface ColumnView {
  key: string;
  title: string;
}

export interface ScanView {
  settings: {
    all_types: boolean;
    types: string[];
    columns: string[];
    summary: string;
  };
  choices: {
    types: string[];
    columns: ColumnView[];
  };
  table: { columns: ColumnView[]; rows: string[][] } | null;
  problem: string | null;
}

export function scanPorts(): Promise<ScanView> {
  return invoke("scan_ports");
}

export function saveScanSettings(allTypes: boolean, types: string[], columns: string[]): Promise<ScanView> {
  return invoke("save_scan_settings", { allTypes, types, columns });
}
