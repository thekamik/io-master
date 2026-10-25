<!--
  The element "API key" of the view Settings, tab AI
  (its commands in src/views/settings/settings.rs): the key of the chosen connection. The
  page never holds a stored key: it learns only whether one is stored, and
  a key typed here is sent once and then cleared from the page. The view
  asks it to read again after the form saved the connection (reload()),
  since a changed address removes the key.
-->
<script lang="ts">
  import { sentence } from "../../lib/api/errors";
  import * as settings from "../../lib/api/settings";
  import ConfirmButton from "../../components/confirm_button/ConfirmButton.svelte";

  let { id }: { id: number | null } = $props();

  let key: settings.ApiKeyView | null = $state(null);
  let typed = $state("");
  let notice: { done: boolean; text: string } | null = $state(null);

  /** Reads whether a key is stored; called by the view after a change. */
  export async function reload() {
    key = null;

    if (id === null) {
      return;
    }

    try {
      key = await settings.apiKeyGet(id);
    } catch (error) {
      notice = { done: false, text: sentence(error) };
    }
  }

  $effect(() => {
    // Read again whenever another connection is chosen.
    void id;
    notice = null;
    reload();
  });

  async function store(event: SubmitEvent) {
    event.preventDefault();

    if (id === null) {
      return;
    }

    try {
      key = await settings.apiKeyStore(id, typed);
      notice = { done: true, text: "The API key was stored." };
    } catch (error) {
      notice = { done: false, text: `The key was not stored: ${sentence(error)}` };
    } finally {
      // The key leaves the page as soon as it was sent.
      typed = "";
    }
  }

  async function remove() {
    if (id === null) {
      return;
    }

    try {
      key = await settings.apiKeyDelete(id);
      notice = { done: true, text: "The API key was deleted." };
    } catch (error) {
      notice = { done: false, text: `The key was not deleted: ${sentence(error)}` };
    }
  }
</script>

<fieldset>
  <legend>API key</legend>
  {#if id === null}
    <p class="note">Save the connection first; then its API key can be stored.</p>
  {:else}
    <p class="note">
      Kept only in your encrypted database, never shown again. It is bound to the address of the endpoint: when the
      scheme, host or port changes, the key is removed.
    </p>
    {#if key?.has_key}
      <p>
        <span class="badge success">An API key is stored</span>
        <ConfirmButton label="Delete the key" onconfirm={remove} />
      </p>
      {#if key.key_unused}
        <p class="problem">No header uses {"{{api_key}}"}, so the key is never sent.</p>
      {/if}
    {:else if key}
      <p>No API key is stored{key.header_uses_key ? ", but a header uses one" : ""}.</p>
    {/if}
    {#if notice}<p class={notice.done ? "done" : "problem"}>{notice.text}</p>{/if}
    <form class="row" onsubmit={store}>
      <label>
        {key?.has_key ? "Replace the key" : "API key"}
        <input type="password" bind:value={typed} autocomplete="off" spellcheck="false" required />
      </label>
      <button type="submit">Store the key</button>
    </form>
  {/if}
</fieldset>
