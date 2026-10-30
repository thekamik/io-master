// The commands of the panel of the chat (src/panels/views/chat/), one
// function per command, named as the command: `chatGet` calls `chat_get`.
// The types mirror the views of those files; a change there needs the same
// change here.

import { Channel, invoke } from "@tauri-apps/api/core";

import type { EndingName } from "./settings";

/** A chat in the list (chats.rs, ChatSummaryView). */
export interface ChatSummaryView {
  id: number;
  title: string;
  agent: string;
  messages: number;
}

/** A message of a chat (chats.rs, MessageView). */
export interface MessageView {
  speaker: "user" | "ai";
  text: string;
  /** Seconds since 1970 (UTC). */
  time: number;
}

/** A chat whole (chats.rs, ChatView). */
export interface ChatView {
  id: number;
  title: string;
  agent: string;
  messages: MessageView[];
}

/** How a question ended (chats.rs, ChatAnswerView). */
export interface ChatAnswerView {
  answer: MessageView | null;
  ending: EndingName;
  error: string | null;
  /** The oldest messages left out of the question, which did not fit with them. */
  leftOut: number;
}

/** Every chat, the newest first. Called by ChatList.svelte. */
export function chatList(): Promise<ChatSummaryView[]> {
  return invoke("chat_list");
}

/** Makes a chat with an agent; gives its id. Called by ChatList.svelte. */
export function chatCreate(title: string, agent: number): Promise<number> {
  return invoke("chat_create", { title, agent });
}

/** One chat whole. Called by Chat.svelte. */
export function chatGet(id: number): Promise<ChatView> {
  return invoke("chat_get", { id });
}

/** Gives a chat a new title. Called by Chat.svelte. */
export function chatRename(id: number, title: string): Promise<void> {
  return invoke("chat_rename", { id, title });
}

/** Asks a question in a chat; `onpiece` gets each piece of the answer as it
 * comes. Called by Chat.svelte. */
export function chatAsk(chat: number, text: string, onpiece: (piece: string) => void): Promise<ChatAnswerView> {
  const onPiece = new Channel<string>();

  onPiece.onmessage = onpiece;

  return invoke("chat_ask", { chat, text, onPiece });
}

/** Asks the answer being written to stop at once. Called by Chat.svelte. */
export function chatStop(): Promise<void> {
  return invoke("chat_stop");
}
