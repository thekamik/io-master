// The element "the AI" of the editor of a catalog: its commands and its
// data (src/views/evaluation_plan/catalog_editor/ai_builder/).

import { Channel, invoke } from "@tauri-apps/api/core";

export interface SourceView {
  id: number;
  title: string;
  page_count: number;
}

export interface PageRange {
  document_id: number;
  first_page: number;
  last_page: number;
}

export type BuildEvent =
  | { event: "part"; index: number; count: number; document: string; first_page: number; last_page: number }
  | { event: "writing"; characters: number }
  | { event: "proposed"; added: number; dropped: string[] }
  | { event: "failed"; message: string }
  | { event: "end"; stopped: boolean };

export function sources(): Promise<SourceView[]> {
  return invoke("ai_builder_sources");
}

/** The AI reads a table of contents and proposes the groups. */
export function groups(uuid: string, range: PageRange, onEvent: (event: BuildEvent) => void): Promise<void> {
  const channel = new Channel<BuildEvent>();
  channel.onmessage = onEvent;

  return invoke("ai_builder_groups", { uuid, range, onEvent: channel });
}

/** The AI reads pages of a group (null: the top) and proposes its controls. */
export function controls(
  uuid: string,
  group: string | null,
  ranges: PageRange[],
  partSize: number,
  onEvent: (event: BuildEvent) => void,
): Promise<void> {
  const channel = new Channel<BuildEvent>();
  channel.onmessage = onEvent;

  return invoke("ai_builder_controls", { uuid, group, ranges, partSize, onEvent: channel });
}

export function stop(): Promise<void> {
  return invoke("ai_builder_stop");
}
