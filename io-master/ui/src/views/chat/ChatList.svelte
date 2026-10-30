<!--
  The list of the chats in the panel of the chat (its commands chat_list
  and chat_create in src/panels/views/chat/chats.rs, agent_list in
  src/panels/views/agents/agent.rs for the choice): every chat with its
  agent, the newest first, and a new chat, made with its agent and opened at
  once (onopen). Read again whenever `changes` grows.
-->
<script lang="ts">
  import { agentList, type AgentSummaryView } from "../../lib/api/agents";
  import { chatCreate, chatList, type ChatSummaryView } from "../../lib/api/chat";
  import { sentence } from "../../lib/api/errors";

  let { open, changes, onopen }: { open: number | null; changes: number; onopen: (chat: number) => void } = $props();

  let chats: ChatSummaryView[] = $state([]);
  let agents: AgentSummaryView[] = $state([]);
  let agent: number | null = $state(null);
  let title = $state("");
  let making = $state(false);
  let problem = $state("");
  let busy = $state(false);

  async function reload() {
    try {
      chats = await chatList();
      agents = await agentList();
      agent = agent ?? agents[0]?.id ?? null;
    } catch (error) {
      problem = sentence(error);
    }
  }

  async function create(event: SubmitEvent) {
    event.preventDefault();

    if (agent === null) {
      return;
    }

    problem = "";
    busy = true;

    try {
      const id = await chatCreate(title, agent);

      title = "";
      making = false;
      await reload();
      onopen(id);
    } catch (error) {
      problem = sentence(error);
    } finally {
      busy = false;
    }
  }

  $effect(() => {
    // Read again when a chat changed (its title, its messages).
    void changes;
    reload();
  });
</script>

<aside class="chat-list">
  <button type="button" onclick={() => (making = !making)}>{making ? "Cancel" : "New chat"}</button>

  {#if problem}<p class="problem">{problem}</p>{/if}

  {#if making}
    {#if agents.length === 0}
      <p class="note">A chat is held with an agent: make one first in the panel Agents.</p>
    {:else}
      <form onsubmit={create}>
        <label>Title <input bind:value={title} required placeholder="The ST of the router" /></label>
        <label>
          Agent
          <select bind:value={agent}>
            {#each agents as one (one.id)}
              <option value={one.id}>{one.name}</option>
            {/each}
          </select>
        </label>
        <button type="submit" disabled={busy}>Make the chat</button>
      </form>
    {/if}
  {/if}

  <ul>
    {#each chats as chat (chat.id)}
      <li>
        <button type="button" class="link" class:active={chat.id === open} onclick={() => onopen(chat.id)}>
          {chat.title}
        </button>
        <span class="note">{chat.agent} · {chat.messages} messages</span>
      </li>
    {:else}
      <li class="note">No chat yet.</li>
    {/each}
  </ul>
</aside>

<style>
  form {
    margin-top: 0.85rem;
  }
</style>
