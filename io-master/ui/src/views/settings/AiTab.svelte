<!--
  The tab AI of the view Settings: the list of connections on the left;
  on the right the chosen connection in the form, its API key and its
  check. This part of the view keeps only what is presentation: which
  connection is chosen, which one the chat uses (from the list), and
  whether the form has changes that are not saved. When the form changed
  the connections, it asks the list and the key to read again.
-->
<script lang="ts">
  import ApiKey from "./ApiKey.svelte";
  import ConnectionCheck from "./ConnectionCheck.svelte";
  import ConnectionForm from "./ConnectionForm.svelte";
  import ConnectionList from "./ConnectionList.svelte";

  /** The chosen connection; "new": a new one; null: none chosen. */
  let chosen: number | "new" | null = $state(null);
  let active: number | null = $state(null);
  let dirty = $state(false);
  let list: ConnectionList | null = $state(null);
  let key: ApiKey | null = $state(null);

  function changed(id: number | null) {
    chosen = id;
    list?.reload();
    key?.reload();
  }
</script>

<div class="chat">
  <ConnectionList bind:this={list} {chosen} busy={false} onchoose={(next) => (chosen = next)} onread={(id) => (active = id)} />

  <section class="conversation ai-config">
    {#if chosen === null}
      <div class="card"><p class="note">
        Choose a connection, or make a new one. A connection describes completely how the program talks to an AI
        service: a local model of the laboratory, or any other service with an HTTP API.
      </p></div>
    {:else}
      <ConnectionForm {chosen} {active} onchanged={changed} ondirty={(value) => (dirty = value)} />
      {#if chosen !== "new"}
        <ConnectionCheck id={chosen} disabled={dirty} />
      {/if}
      <ApiKey bind:this={key} id={chosen === "new" ? null : chosen} />
    {/if}
  </section>
</div>
