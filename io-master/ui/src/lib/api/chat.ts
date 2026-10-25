// The commands of the view Chat, one function per command of its entry in
// the program (src/views/chat/chat.rs), named as the command:
// `chatListGet` calls `chat_list_get`.
//
// The types mirror the view.rs of the elements (src/views/chat/); a change
// there needs the same change here.

import { Channel, invoke } from "@tauri-apps/api/core";

/** The connection the chat uses (src/views/chat/connection_line/view.rs). */
export interface ConnectionLineView {
  name: string;
  /** Who writes the replies: the model, or the name of the connection. */
  writer: string;
}

/** A chat in the list (src/views/chat/chat_list/view.rs). */
export interface ChatSummaryView {
  id: number;
  title: string;
  /** Local time it was started: "2026-10-14 15:04". */
  created: string;
}

/** A message of a chat (src/views/chat/conversation/view.rs). */
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
  /** The agent with knowledge of the chat (step 57.6); null: none. */
  agent: number | null;
}

/** An event of a reply being written. */
export type ReplyEvent =
  | { event: "chat"; id: number; title: string }
  | { event: "text"; text: string }
  | {
      event: "end";
      ending: "complete" | "cut" | "stopped" | "refused" | "failed";
      notice: string | null;
    };

// ── When the view opens ──────────────────────────────────────────────

/** The connection the chat uses now; null when none is chosen. Called by ConnectionLine.svelte when the view opens. */
export function connectionLineGet(): Promise<ConnectionLineView | null> {
  return invoke("connection_line_get");
}

/** The chats, the newest first. Called by ChatList.svelte when the view opens and when the chats changed. */
export function chatListGet(): Promise<ChatSummaryView[]> {
  return invoke("chat_list_get");
}

// ── The actions of the elements ──────────────────────────────────────

/** A chat with its messages. Called by Conversation.svelte when a chat is chosen. */
export function conversationOpen(chatId: number): Promise<ChatView> {
  return invoke("conversation_open", { chatId });
}

/**
 * Sends a message (`chatId` null: a new chat). The events of the reply arrive in `onEvent` while it is
 * written; the promise ends when the reply is stored. Called by Conversation.svelte, "Send".
 */
export function conversationSend(
  chatId: number | null,
  text: string,
  agent: number | null,
  onEvent: (event: ReplyEvent) => void,
): Promise<void> {
  const channel = new Channel<ReplyEvent>();
  channel.onmessage = onEvent;
  return invoke("conversation_send", { chatId, text, agent, onEvent: channel });
}

/** Stops the reply being written. Called by Conversation.svelte, "Stop". */
export function conversationStop(): Promise<void> {
  return invoke("conversation_stop");
}

/** Deletes a chat with its messages. Called by Conversation.svelte, "Delete". */
export function conversationDelete(chatId: number): Promise<void> {
  return invoke("conversation_delete", { chatId });
}
