<!--
  One chat in the panel of the chat (its commands chat_get, chat_rename,
  chat_ask and chat_stop in src/panels/views/chat/chats.rs): its messages,
  the field of the next question, and the answer shown piece by piece while
  it is written, with "Stop". When the oldest messages were left out of a
  question that did not fit with them, it says how many. The text of a message is shown as plain text,
  never as HTML (user, 2026-10-30). After every question the chat is read
  again, as the program kept it, and the list is told (onchanged).
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { chatAsk, chatGet, chatRename, chatStop, type ChatView } from "../../lib/api/chat";
  import { sentence } from "../../lib/api/errors";

  let { chat, onchanged }: { chat: number; onchanged: () => void } = $props();

  let view: ChatView | null = $state(null);
  let question = $state("");
  /** The question being answered, and the answer so far; null when none. */
  let asked: string | null = $state(null);
  let written = $state("");
  let stopping = $state(false);
  let renaming: string | null = $state(null);
  let problem = $state("");
  /** How many old messages the last question left out. */
  let leftOut = $state(0);
  /** The end of the chat, below the field of the question: brought into
   * sight when the chat is read and while the answer grows. */
  let end: HTMLElement | undefined = $state();

  $effect(() => {
    void view;
    void written;
    end?.scrollIntoView({ block: "end" });
  });

  async function reload() {
    try {
      view = await chatGet(chat);
    } catch (error) {
      problem = sentence(error);
    }
  }

  async function ask(event: SubmitEvent) {
    event.preventDefault();
    problem = "";
    asked = question;
    question = "";
    written = "";
    stopping = false;

    try {
      const ended = await chatAsk(chat, asked, (piece) => (written += piece));

      leftOut = ended.leftOut;

      if (ended.error !== null) {
        problem = sentence(ended.error);
      }
    } catch (error) {
      // Refused before it was sent (too long, no key): the question is given
      // back to be changed.
      question = asked ?? "";
      problem = sentence(error);
    } finally {
      asked = null;
      await reload();
      onchanged();
    }
  }

  async function stop() {
    stopping = true;

    try {
      await chatStop();
    } catch (error) {
      problem = sentence(error);
    }
  }

  async function rename(event: SubmitEvent) {
    event.preventDefault();

    if (renaming === null) {
      return;
    }

    try {
      await chatRename(chat, renaming);
      renaming = null;
      await reload();
      onchanged();
    } catch (error) {
      problem = sentence(error);
    }
  }

  /** Ctrl+Enter sends the question. */
  function keydown(event: KeyboardEvent) {
    if (event.key === "Enter" && event.ctrlKey && asked === null && question.trim() !== "") {
      (event.currentTarget as HTMLTextAreaElement).form?.requestSubmit();
    }
  }

  onMount(reload);
</script>

<section class="conversation">
  {#if view !== null}
    {#if renaming === null}
      <h2>
        {view.title}
        <button type="button" class="ghost small" onclick={() => (renaming = view?.title ?? "")}>Rename</button>
      </h2>
    {:else}
      <form class="rename" onsubmit={rename}>
        <input bind:value={renaming} required />
        <button type="submit">Save</button>
        <button type="button" class="ghost" onclick={() => (renaming = null)}>Cancel</button>
      </form>
    {/if}

    <p class="status">Agent <strong>{view.agent}</strong></p>

    <div class="messages">
      {#each view.messages as message, index (index)}
        <div class="message" class:evaluator={message.speaker === "user"} class:model={message.speaker === "ai"}>
          <header>{message.speaker === "user" ? "You" : view.agent}</header>
          <div class="text">{message.text}</div>
        </div>
      {/each}

      {#if asked !== null}
        <div class="message evaluator">
          <header>You</header>
          <div class="text">{asked}</div>
        </div>
        <div class="message model">
          <header>{view.agent}</header>
          <div class="text">{written}</div>
        </div>
      {/if}
    </div>
  {/if}

  {#if leftOut > 0}
    <p class="note">The {leftOut} oldest messages were left out of the last question: they do not fit into the model.</p>
  {/if}
  {#if problem}<p class="problem">{problem}</p>{/if}

  <form class="ask" onsubmit={ask}>
    <textarea
      bind:value={question}
      rows="3"
      placeholder="Your question (Ctrl+Enter sends it)"
      disabled={asked !== null}
      onkeydown={keydown}
    ></textarea>
    <div class="buttons">
      {#if asked === null}
        <button type="submit" disabled={question.trim() === ""}>Send</button>
      {:else}
        <button type="button" class="ghost" disabled={stopping} onclick={stop}>{stopping ? "Stopping…" : "Stop"}</button>
      {/if}
    </div>
  </form>
  <div class="end" bind:this={end}></div>
</section>

<style>
  /* As far from the bottom as the field of the question stays (the padding
     of the panel), so that the field never covers the end of the answer. */
  .end {
    scroll-margin-bottom: 2.5rem;
  }

  form.rename {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    margin-bottom: 0.75rem;
  }

  form.rename input {
    margin: 0;
  }
</style>
