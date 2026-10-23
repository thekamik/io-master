// The element "list of chats" of the view Chat: its command and its data
// (src/views/chat/chat_list/).
//
// The types mirror src/views/chat/chat_list/view.rs.

import { invoke } from "@tauri-apps/api/core";

/** A chat in the list. */
export interface ChatSummaryView {
  id: number;
  title: string;
  /** Local time it was started: "2026-10-14 15:04". */
  created: string;
}

/** The chats, the newest first. */
export function get(): Promise<ChatSummaryView[]> {
  return invoke("chat_list_get");
}
