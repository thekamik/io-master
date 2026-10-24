// The command of the component "tree of a catalog" (src/components/catalog_tree/catalog_tree.rs),
// named as the command: `catalogTreeGet` calls `catalog_tree_get`. Used by
// the editor of a catalog (Converter) and the browser of the standards
// (Evaluation plan). The types mirror src/components/catalog_tree/view.rs.

import { invoke } from "@tauri-apps/api/core";

/** A row of the tree: a group or a control. */
export interface RowView {
  kind: "group" | "control";
  id: string | null;
  title: string;
  level: number;
}

/** The tree of a catalog. */
export interface CatalogTreeView {
  title: string;
  version: string;
  last_modified: string;
  rows: RowView[];
}

/** Called by CatalogTree.svelte, when it opens and after every change of the catalog. */
export function catalogTreeGet(uuid: string): Promise<CatalogTreeView> {
  return invoke("catalog_tree_get", { uuid });
}
