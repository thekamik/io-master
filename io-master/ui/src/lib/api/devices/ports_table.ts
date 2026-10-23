// The element "table of ports" of the view Devices: its command and its
// data (src/views/devices/ports_table/).
//
// The types mirror src/views/devices/ports_table/view.rs; a change there
// needs the same change here.

import { invoke } from "@tauri-apps/api/core";

/** A column of the table (src/views/devices/column_view.rs). */
export interface ColumnView {
  key: string;
  title: string;
}

/** The table of ports. */
export interface PortsTableView {
  columns: ColumnView[];
  /** One row per port, one text per column; empty when the scan failed. */
  rows: string[][];
  /** Why the ports could not be scanned; null when the scan worked. */
  problem: string | null;
}

/** Scans the serial ports with the stored settings. */
export function scan(): Promise<PortsTableView> {
  return invoke("ports_table_scan");
}
