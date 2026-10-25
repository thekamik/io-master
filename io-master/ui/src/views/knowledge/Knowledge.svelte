<!--
  The view Knowledge (RAG) (step 57 of docs/todo.md; its commands in
  src/views/knowledge/knowledge.rs), the configurator of the knowledge of
  the AI, kept apart from the views of the evaluation (user, 2026-10-25),
  in two tabs:

  - Knowledge bases: the list (KnowledgeList) or one base (KnowledgeBase)
    with its sources and its search (KnowledgeSearch);
  - Agents: the list (AgentList) or one agent (AgentForm), whose bases are
    ticked in its form, with its proving ground (ProvingGround).

  Which tab and which base or agent is open is presentation, kept here.
-->
<script lang="ts">
  import AgentForm from "./AgentForm.svelte";
  import AgentList from "./AgentList.svelte";
  import KnowledgeBase from "./KnowledgeBase.svelte";
  import KnowledgeList from "./KnowledgeList.svelte";

  type Tab = "bases" | "agents";

  const tabs: { id: Tab; title: string }[] = [
    { id: "bases", title: "Knowledge bases" },
    { id: "agents", title: "Agents" },
  ];

  let tab: Tab = $state("bases");
  /** The base open, if any. */
  let base: number | null = $state(null);
  /** The agent open, if any. */
  let agent: number | null = $state(null);
</script>

<div class="tabs">
  {#each tabs as item (item.id)}
    <button type="button" class:active={tab === item.id} onclick={() => (tab = item.id)}>{item.title}</button>
  {/each}
</div>

{#if tab === "bases"}
  {#if base !== null}
    <KnowledgeBase id={base} onclose={() => (base = null)} />
  {:else}
    <KnowledgeList onopen={(id) => (base = id)} />
  {/if}
{:else if agent !== null}
  <AgentForm id={agent} onclose={() => (agent = null)} />
{:else}
  <AgentList onopen={(id) => (agent = id)} />
{/if}
