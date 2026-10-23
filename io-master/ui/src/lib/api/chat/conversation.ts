// The element "conversation" of the view Chat: its commands and its data
// (src/views/chat/conversation/).
//
// The types mirror src/views/chat/conversation/view.rs.

import { Channel, invoke } from "@tauri-apps/api/core";

export interface MessageView {
  speaker: "evaluator" | "model";
  who: string;
  text: string;
  created: string;
  notice: string | null;
}

/** An open chat. */
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

/** A chat with its messages. */
export function open(chatId: number): Promise<ChatView> {
  return invoke("conversation_open", { chatId });
}

/**
 * Sends a message (`chatId` null: a new chat). The events of the reply
 * arrive in `onEvent` while it is written; the promise ends when the
 * reply is stored.
 */
export function send(chatId: number | null, text: string, onEvent: (event: ReplyEvent) => void): Promise<void> {
  const channel = new Channel<ReplyEvent>();

  channel.onmessage = onEvent;

  return invoke("conversation_send", { chatId, text, onEvent: channel });
}

/** Stops the reply being written. */
export function stop(): Promise<void> {
  return invoke("conversation_stop");
}

/** Deletes a chat with its messages. */
export function remove(chatId: number): Promise<void> {
  return invoke("conversation_delete", { chatId });
}
