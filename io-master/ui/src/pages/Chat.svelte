<!--
  The chat page: the chats of the evaluator with the model of the
  active AI connection, which is set up and chosen on the page AI
  config.

  A reply is shown while it is written (sendMessage in lib/api.ts)
  and can be stopped. Every message and reply is stored by the
  program; this page only shows them. Text is always set as text,
  never as HTML, so a reply can never become markup or a script.
-->
<script lang="ts">
  import { onMount, tick } from "svelte";

  import {
    aiConfig,
    deleteChat,
    listChats,
    openChat,
    sendMessage,
    sentence,
    stopReply,
    type ChatSummaryView,
    type ConnectionView,
    type MessageView,
  } from "../lib/api";

  /** The active connection; null when none is chosen. */
  let connection = $state<ConnectionView | null>(null);
  /** Whether the AI config was read (to tell "none" from "not yet"). */
  let configRead = $state(false);
  let chats: ChatSummaryView[] = $state([]);
  let problem = $state("");

  /** The open chat; null: a new chat. */
  let chatId: number | null = $state(null);
  let title = $state("New chat");
  let messages: MessageView[] = $state([]);

  let text = $state("");
  let writing = $state(false);

  /** Who writes the replies, as the page names it. */
  let writer = $derived(connection ? connection.model || connection.name : "the model");

  /** The element with the messages, scrolled to the end. */
  let list: HTMLElement | undefined = $state();

  async function load() {
    try {
      const config = await aiConfig();

      connection = config.connections.find((candidate) => candidate.id === config.active) ?? null;
      configRead = true;
      chats = await listChats();
    } catch (error) {
      problem = `Your chats cannot be read: ${sentence(error)}`;
    }
  }

  onMount(load);

  async function scrollDown() {
    await tick();
    list?.lastElementChild?.scrollIntoView({ block: "end" });
  }

  function newChat() {
    chatId = null;
    title = "New chat";
    messages = [];
    problem = "";
  }

  async function open(id: number) {
    problem = "";

    try {
      const chat = await openChat(id);

      chatId = chat.id;
      title = chat.title;
      messages = chat.messages;
      await scrollDown();
    } catch (error) {
      problem = sentence(error);
    }
  }

  async function remove() {
    if (chatId === null) {
      return;
    }

    try {
      await deleteChat(chatId);
      newChat();
      chats = await listChats();
    } catch (error) {
      problem = `The chat was not deleted: ${sentence(error)}`;
    }
  }

  /** The current time as the program shows it: "2026-10-14 15:04". */
  function now(): string {
    const date = new Date();
    const two = (number: number) => String(number).padStart(2, "0");

    return (
      `${date.getFullYear()}-${two(date.getMonth() + 1)}-${two(date.getDate())} ` +
      `${two(date.getHours())}:${two(date.getMinutes())}`
    );
  }

  async function send(event?: SubmitEvent) {
    event?.preventDefault();

    const message = text.trim();

    if (!message || writing) {
      return;
    }

    problem = "";
    writing = true;

    messages.push({ speaker: "evaluator", who: "You", text: message, created: now(), notice: null });
    messages.push({ speaker: "model", who: writer, text: "", created: now(), notice: null });

    // The reply is the last message; the events fill it in.
    const reply = messages[messages.length - 1]!;
    await scrollDown();

    try {
      await sendMessage(chatId, message, (replyEvent) => {
        if (replyEvent.event === "chat") {
          chatId = replyEvent.id;
          title = replyEvent.title;
          text = "";
        } else if (replyEvent.event === "text") {
          reply.text += replyEvent.text;
          scrollDown();
        } else {
          reply.notice = replyEvent.notice;
        }
      });

      chats = await listChats();
    } catch (error) {
      // Refused before the reply started (no key, empty message):
      // the message was not stored, so it is taken back.
      messages.splice(messages.length - 2, 2);
      problem = sentence(error);
    } finally {
      writing = false;
    }
  }

  /** Enter sends, Shift+Enter starts a new line. */
  function keydown(event: KeyboardEvent) {
    if (event.key === "Enter" && !event.shiftKey && !event.isComposing) {
      event.preventDefault();
      send();
    }
  }
</script>

{#if configRead}
  {#if connection}
    <p class="status" title="Another connection is chosen in Settings, tab AI">
      <strong>{connection.name}</strong> · {writer}
    </p>
  {:else}
    <p class="problem">No AI connection is chosen. Set one up in Settings, tab AI, to use the chat.</p>
  {/if}
{/if}

{#if problem}<p class="problem">{problem}</p>{/if}

<div class="chat">
  <aside class="chat-list">
    <button class="primary" onclick={newChat} disabled={writing}>New chat</button>
    <ul>
      {#each chats as chat (chat.id)}
        <li>
          <button class="link" class:active={chat.id === chatId} onclick={() => open(chat.id)} disabled={writing}>
            {chat.title}
          </button>
          <span class="note">{chat.created}</span>
        </li>
      {/each}
    </ul>
  </aside>

  <section class="conversation">
    <h2>
      {title}
      {#if chatId !== null}<button class="small danger" onclick={remove} disabled={writing}>Delete this chat</button>{/if}
    </h2>

    <div class="messages" bind:this={list}>
      {#each messages as message, index (index)}
        <article class="message {message.speaker}">
          <header>{message.who} · {message.created}</header>
          <div class="text">{message.text}</div>
          {#if message.notice}<p class="ending">{message.notice}</p>{/if}
        </article>
      {/each}
    </div>

    <form class="ask" onsubmit={send}>
      <textarea
        bind:value={text}
        onkeydown={keydown}
        rows="4"
        maxlength="100000"
        placeholder="Write to {writer}"
      ></textarea>
      <div class="buttons">
        <button type="submit" disabled={writing}>Send</button>
        {#if writing}
          <button type="button" class="danger" onclick={stopReply}>Stop</button>
          <span class="note">The model is writing…</span>
        {/if}
      </div>
    </form>
  </section>
</div>
