<!--
  The view Settings (src/views/settings/): the settings of the program, in
  tabs (those of the user, the account and the encryption, are on the page
  User, src/views/users/):

  - AI: the connections to AI services (AiTab.svelte);
  - Program: what this installation is and how it is protected (later the
    TPM factor, step 8).

  Which tab is open is presentation, kept here.
-->
<script lang="ts">
  import type { ProgramView } from "../../lib/api/shell";
  import AiTab from "./AiTab.svelte";
  import ProgramTab from "./ProgramTab.svelte";

  let { app }: { app: ProgramView } = $props();

  type Tab = "ai" | "program";

  const tabs: { id: Tab; title: string }[] = [
    { id: "ai", title: "AI" },
    { id: "program", title: "Program" },
  ];

  let tab: Tab = $state("ai");
</script>

<div class="tabs">
  {#each tabs as item (item.id)}
    <button type="button" class:active={tab === item.id} onclick={() => (tab = item.id)}>{item.title}</button>
  {/each}
</div>

{#if tab === "ai"}
  <AiTab />
{:else}
  <ProgramTab {app} />
{/if}
