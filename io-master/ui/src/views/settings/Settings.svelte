<!--
  The settings of the AI (their commands in src/panels/views/settings/), in
  two tabs: the model Integration (user, 2026-10-29): the AI services the
  program asks, their keys, the active one and the check of the connection;
  and the model CustomFormat (user, 2026-11-14): the JSON of the question
  and of the answer of a service that speaks neither protocol of the
  program. The agents, their knowledge and their journals have a panel of
  their own.
-->
<script lang="ts">
  import CustomFormats from "./CustomFormats.svelte";
  import IntegrationList from "./IntegrationList.svelte";

  let tab: "integrations" | "formats" = $state("integrations");
</script>

<div class="page-head">
  <h1>AI settings</h1>
  <p>The AI services the program asks: their addresses, models and keys, and the formats of those that speak their own JSON.</p>
</div>

<div class="tabs">
  <button type="button" class:active={tab === "integrations"} onclick={() => (tab = "integrations")}>Integrations</button>
  <button type="button" class:active={tab === "formats"} onclick={() => (tab = "formats")}>Custom formats</button>
</div>

{#if tab === "integrations"}
  <IntegrationList />
{:else}
  <CustomFormats />
{/if}
