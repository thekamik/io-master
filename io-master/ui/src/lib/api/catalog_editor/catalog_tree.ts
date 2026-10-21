// The element "tree" of the editor of a catalog: its command and its data
// (src/views/catalog_editor/catalog_tree/).

import { invoke } from "@tauri-apps/api/core";

export interface RowView {
  kind: "group" | "control";
  id: string | null;
  title: string;
  level: number;
}

export interface CatalogTreeView {
  title: string;
  version: string;
  last_modified: string;
  rows: RowView[];
}

export function get(uuid: string): Promise<CatalogTreeView> {
  return invoke("catalog_tree_get", { uuid });
}
