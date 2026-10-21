// The element "object" of the editor of a catalog: its commands and its
// data (src/views/catalog_editor/object_form/).

import { invoke } from "@tauri-apps/api/core";

import type { PartRowView } from "./part_rows";

export interface ObjectView {
  kind: "group" | "control";
  id: string;
  title: string;
  parts: PartRowView[];
  /** Its own fields as JSON of OSCAL (without what it holds). */
  fields: Record<string, unknown>;
}

export function get(uuid: string, id: string): Promise<ObjectView> {
  return invoke("object_form_get", { uuid, id });
}

export function saveGroup(uuid: string, id: string, group: unknown): Promise<void> {
  return invoke("object_form_save_group", { uuid, id, group });
}

export function saveControl(uuid: string, id: string, control: unknown): Promise<void> {
  return invoke("object_form_save_control", { uuid, id, control });
}

export function remove(uuid: string, id: string): Promise<void> {
  return invoke("object_form_delete", { uuid, id });
}
