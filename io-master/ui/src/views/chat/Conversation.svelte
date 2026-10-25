<!--
  The element "conversation" of the view Chat
  (its commands conversation_* in src/views/chat/chat.rs): the open chat, a new message with its
  reply shown while it is written (it can be stopped), and the deletion.

  The view says which chat is open (chatId; null: a new chat). When this
  element changes the chats (a message sent, a chat deleted) it tells the
  view (onchanged) with the chat that is open now; the view then asks the
  list to read the chats again. Every message and reply is stored by the
  program; this element only shows them. Text is always set as text,
  never as HTML, so a reply can never become markup or a script.
-->
<script lang="ts">
  import { tick } from "svelte";

  import { sentence } from "../../lib/api/errors";
  import * as chat from "../../lib/api/chat";
  import AgentPicker from "../../components/knowledge_agent/AgentPicker.svelte";
  import ConfirmButton from "../../components/confirm_button/ConfirmButton.svelte";

  let {
    chatId,
    writer,
    onchanged,
    onwriting,
  }: {
    chatId: number | null;
    writer: string;
    onchanged: (chatId: number | null) => void;
    onwriting: (writing: boolean) => void;
  } = $props();

  /** The chat shown; undefined until the first is shown. Not reactive on purpose. */
  let shown: number | null | undefined = undefined;
  let title = $state("New chat");
  /** The agent with knowledge of the chat (step 57.6); sent with every message, kept by Rust. */
  let agent: number | null = $state(null);
  let messages: chat.MessageView[] = $state([]);
  let problem = $state("");

  let text = $state("");
  let writing = $state(false);

  /** The element with the messages, scrolled to the end. */
  let list: HTMLElement | undefined = $state();

  // Shows the chat the view chose, unless it is already shown (a new
  // chat that this element just made).
  $effect(() => {
    if (chatId !== shown) {
      show(chatId);
    }
  });

  async function show(id: number | null) {
    shown = id;
    problem = "";

    if (id === null) {
      title = "New chat";
      messages = [];
      agent = null;
      return;
    }

    try {
      const opened = await chat.conversationOpen(id);

      title = opened.title;
      messages = opened.messages;
      agent = opened.agent;
      await scrollDown();
    } catch (error) {
      problem = sentence(error);
    }
  }

  async function scrollDown() {
    await tick();
    list?.lastElementChild?.scrollIntoView({ block: "end" });
  }

  async function remove() {
    if (shown === null || shown === undefined) {
      return;
    }

    try {
      await chat.conversationDelete(shown);
      await show(null);
      onchanged(null);
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

  /** Puts one event of the reply into `reply`, the last message. */
  function apply(event: chat.ReplyEvent, reply: chat.MessageView) {
    if (event.event === "chat") {
      shown = event.id;
      title = event.title;
      text = "";
    } else if (event.event === "text") {
      reply.text += event.text;
      scrollDown();
    } else {
      reply.notice = event.notice;
    }
  }

  async function send(event?: SubmitEvent) {
    event?.preventDefault();

    const message = text.trim();

    if (!message || writing) {
      return;
    }

    problem = "";
    writing = true;
    onwriting(true);

    messages.push({ speaker: "evaluator", who: "You", text: message, created: now(), notice: null });
    messages.push({ speaker: "model", who: writer, text: "", created: now(), notice: null });

    // The reply is the last message; the events fill it in.
    const reply = messages[messages.length - 1]!;
    await scrollDown();

    try {
      await chat.conversationSend(shown ?? null, message, agent, (replyEvent) => apply(replyEvent, reply));
      onchanged(shown ?? null);
    } catch (error) {
      // Refused before the reply started (no key, empty message): the
      // message was not stored, so it is taken back.
      messages.splice(messages.length - 2, 2);
      problem = sentence(error);
    } finally {
      writing = false;
      onwriting(false);
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

<section class="conversation">
  <h2>
    {title}
    {#if chatId !== null}<ConfirmButton label="Delete this chat" disabled={writing} onconfirm={remove} />{/if}
  </h2>

  <AgentPicker value={agent} onchange={(chosen) => (agent = chosen)} />

  {#if problem}<p class="problem">{problem}</p>{/if}

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
    <textarea bind:value={text} onkeydown={keydown} rows="4" maxlength="100000" placeholder="Write to {writer}"
    ></textarea>
    <div class="buttons">
      <button type="submit" disabled={writing}>Send</button>
      {#if writing}
        <button type="button" class="danger" onclick={chat.conversationStop}>Stop</button>
        <span class="note">The model is writing…</span>
      {/if}
    </div>
  </form>
</section>
