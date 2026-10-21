<!--
  The view Chat (src/views/chat/): the connection the chat uses, the list
  of chats and the open chat. The view keeps only what is presentation:
  which chat is open, who writes the replies, and whether a reply is being
  written (the list is then not usable). When the conversation changed
  the chats, the view asks the list to read them again.
-->
<script lang="ts">
  import ChatList from "./ChatList.svelte";
  import ConnectionLine from "./ConnectionLine.svelte";
  import Conversation from "./Conversation.svelte";

  /** The open chat; null: a new chat. */
  let chosen: number | null = $state(null);
  let writer = $state("the model");
  let writing = $state(false);
  let list: ChatList | null = $state(null);

  function changed(chatId: number | null) {
    chosen = chatId;
    list?.reload();
  }
</script>

<ConnectionLine onread={(line) => (writer = line?.writer ?? "the model")} />

<div class="chat">
  <ChatList bind:this={list} {chosen} busy={writing} onchoose={(chatId) => (chosen = chatId)} />
  <Conversation chatId={chosen} {writer} onchanged={changed} onwriting={(value) => (writing = value)} />
</div>
