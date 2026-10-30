<!--
  The panel of the chat (its commands in src/panels/views/chat/chats.rs):
  the chats on the left (ChatList), the chat opened on the right
  (Conversation). The user chooses an agent and talks with the model of its
  integration, which gets the knowledge of the agent with every question
  (user, 2026-10-30). Which chat is open is presentation only: it stays in
  the page.
-->
<script lang="ts">
  import ChatList from "./ChatList.svelte";
  import Conversation from "./Conversation.svelte";

  let open: number | null = $state(null);
  /** Counts the changes of a chat, so that the list is read again. */
  let changes = $state(0);
</script>

<div class="page-head">
  <h1>Chat</h1>
  <p>Talk with an agent: the model of its integration answers, with the knowledge of the agent.</p>
</div>

<div class="chat">
  <ChatList {open} {changes} onopen={(id) => (open = id)} />

  {#if open !== null}
    {#key open}
      <Conversation chat={open} onchanged={() => (changes += 1)} />
    {/key}
  {:else}
    <section class="conversation">
      <p class="note">Open a chat, or make a new one with an agent.</p>
    </section>
  {/if}
</div>
