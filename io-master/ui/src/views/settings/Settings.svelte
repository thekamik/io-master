<!--
  The view Settings (src/views/settings/), in tabs:

  - User: the password of the evaluator (later the factors of the person,
    step 9 of docs/todo.md);
  - AI: the connections to AI services (AiTab.svelte);
  - Program: what this installation is and how it is protected (later the
    TPM factor, the secret of the application and the removal of the
    installation, steps 8 and 13).

  Which tab is open is presentation, kept here.
-->
<script lang="ts">
  import type { ProgramView } from "../../lib/api/shell";
  import AiTab from "./AiTab.svelte";
  import PasswordForm from "./PasswordForm.svelte";
  import ProgramTab from "./ProgramTab.svelte";

  let { app }: { app: ProgramView } = $props();

  type Tab = "user" | "ai" | "program";

  const tabs: { id: Tab; title: string }[] = [
    { id: "user", title: "User" },
    { id: "ai", title: "AI" },
    { id: "program", title: "Program" },
  ];

  let tab: Tab = $state("user");
</script>

<div class="tabs">
  {#each tabs as item (item.id)}
    <button type="button" class:active={tab === item.id} onclick={() => (tab = item.id)}>{item.title}</button>
  {/each}
</div>

{#if tab === "user"}
  <PasswordForm user={app.user} />
{:else if tab === "ai"}
  <AiTab />
{:else}
  <ProgramTab {app} />
{/if}
