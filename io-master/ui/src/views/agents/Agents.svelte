<!--
  The panel of the agents (its commands in src/panels/views/agents/): two
  tabs, the two models the user makes here. Agents: an agent is a layer of
  knowledge over the model of an integration; an agent opened shows its
  editor (AgentEditor) with its journal. Knowledge bases: the bases shared
  by the agents and their sources; a base opened shows its editor
  (BaseEditor). Which tab and screen are shown is presentation only: it
  stays in the page.
-->
<script lang="ts">
  import AgentEditor from "./AgentEditor.svelte";
  import AgentList from "./AgentList.svelte";
  import BaseEditor from "./BaseEditor.svelte";
  import KnowledgeBases from "./KnowledgeBases.svelte";

  let tab: "agents" | "bases" = $state("agents");
  let openAgent: number | null = $state(null);
  let openBase: number | null = $state(null);
</script>

<div class="page-head">
  <h1>Agents</h1>
  <p>The agents that work on the work units, and the knowledge they give to the model of their integration.</p>
</div>

{#if openAgent !== null}
  <AgentEditor agent={openAgent} onback={() => (openAgent = null)} />
{:else if openBase !== null}
  <BaseEditor base={openBase} onback={() => (openBase = null)} />
{:else}
  <div class="tabs">
    <button type="button" class:active={tab === "agents"} onclick={() => (tab = "agents")}>Agents</button>
    <button type="button" class:active={tab === "bases"} onclick={() => (tab = "bases")}>Knowledge bases</button>
  </div>

  {#if tab === "agents"}
    <AgentList onopen={(id) => (openAgent = id)} />
  {:else}
    <KnowledgeBases onopen={(id) => (openBase = id)} />
  {/if}
{/if}
