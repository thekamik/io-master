// The chat (serializer/chat_view.rs; tauri_ui/commands/chats.rs).
//
// The types mirror the views of src/tauri_ui/serializer/; a change
// there needs the same change here.

import { Channel, invoke } from "@tauri-apps/api/core";

export interface ChatSummaryView {
  id: number;
  title: string;
  created: string;
}

export interface MessageView {
  speaker: "evaluator" | "model";
  who: string;
  text: string;
  created: string;
  notice: string | null;
}

export interface ChatView {
  id: number;
  title: string;
  messages: MessageView[];
}

/** An event of a reply being written. */
export type ReplyEvent =
  | { event: "chat"; id: number; title: string }
  | { event: "text"; text: string }
  | { event: "end"; ending: "complete" | "cut" | "stopped" | "refused" | "failed"; notice: string | null };

export function listChats(): Promise<ChatSummaryView[]> {
  return invoke("list_chats");
}

export function openChat(chatId: number): Promise<ChatView> {
  return invoke("open_chat", { chatId });
}

export function deleteChat(chatId: number): Promise<void> {
  return invoke("delete_chat", { chatId });
}

/**
 * Sends a message (`chatId` null: a new chat). The events of the
 * reply arrive in `onEvent` while it is written; the promise ends
 * when the reply is stored.
 */
export function sendMessage(chatId: number | null, text: string, onEvent: (event: ReplyEvent) => void): Promise<void> {
  const channel = new Channel<ReplyEvent>();

  channel.onmessage = onEvent;

  return invoke("send_message", { chatId, text, onEvent: channel });
}

/** Stops the reply being written. */
export function stopReply(): Promise<void> {
  return invoke("stop_reply");
}
