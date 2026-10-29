<!--
  The settings of the AI (their commands in src/panels/views/settings/):
  three tabs. Integrations: the AI services the program asks, their keys and
  the check of the connection. Instructions: the fixed part of the questions
  of each task. Journal: every question asked and its answer. Which tab is
  shown is presentation only: it stays in the page.
-->
<script lang="ts">
  import InstructionsTab from "./InstructionsTab.svelte";
  import IntegrationsTab from "./IntegrationsTab.svelte";
  import JournalTab from "./JournalTab.svelte";

  let tab: "integrations" | "instructions" | "journal" = $state("integrations");
</script>

<div class="page-head">
  <h1>AI settings</h1>
  <p>The AI services the program asks, the instructions of its tasks, the journal of every question.</p>
</div>

<div class="tabs">
  <button type="button" class:active={tab === "integrations"} onclick={() => (tab = "integrations")}>Integrations</button>
  <button type="button" class:active={tab === "instructions"} onclick={() => (tab = "instructions")}>Instructions</button>
  <button type="button" class:active={tab === "journal"} onclick={() => (tab = "journal")}>Journal</button>
</div>

{#if tab === "integrations"}
  <IntegrationsTab />
{:else if tab === "instructions"}
  <InstructionsTab />
{:else}
  <JournalTab />
{/if}
