<!--
  The element "list of chats" of the view Chat (its command chat_list_get in src/views/chat/chat.rs).
  Choosing a chat and "New chat" only tell the view (onchoose); the view
  opens the chat in the conversation. The view asks for the list again
  when the conversation changed the chats (reload(), below).
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../../lib/api/errors";
  import * as chat from "../../lib/api/chat";

  let {
    chosen,
    busy,
    onchoose,
  }: { chosen: number | null; busy: boolean; onchoose: (chatId: number | null) => void } = $props();

  let chats: chat.ChatSummaryView[] = $state([]);
  let problem = $state("");

  /** Reads the chats again; called by the view after a change. */
  export async function reload() {
    try {
      chats = await chat.chatListGet();
      problem = "";
    } catch (error) {
      problem = `Your chats cannot be read: ${sentence(error)}`;
    }
  }

  onMount(reload);
</script>

<aside class="chat-list">
  <button class="primary" onclick={() => onchoose(null)} disabled={busy}>New chat</button>
  {#if problem}<p class="problem">{problem}</p>{/if}
  <ul>
    {#each chats as chat (chat.id)}
      <li>
        <button class="link" class:active={chat.id === chosen} onclick={() => onchoose(chat.id)} disabled={busy}>
          {chat.title}
        </button>
        <span class="note">{chat.created}</span>
      </li>
    {/each}
  </ul>
</aside>
