// The element "new object" of the editor of a catalog: its commands
// (src/views/catalog_editor/new_object/).

import { invoke } from "@tauri-apps/api/core";

/** Where a new object goes. */
export type Place = { kind: "top" } | { kind: "group"; id: string } | { kind: "control"; id: string };

export function addGroup(uuid: string, place: Place, group: unknown): Promise<void> {
  return invoke("new_object_add_group", { uuid, place, group });
}

export function addControl(uuid: string, place: Place, control: unknown): Promise<void> {
  return invoke("new_object_add_control", { uuid, place, control });
}
