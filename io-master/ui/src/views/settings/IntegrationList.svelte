<!--
  The integrations of the settings of the AI (their commands
  integration_list and integration_activate in
  src/panels/views/settings/integrations.rs): every integration, whether it
  has a key, the active one (asked where no agent names another). An
  integration opened, or a new one, is edited in IntegrationForm below the
  list.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../../lib/api/errors";
  import { integrationActivate, integrationList, type IntegrationView } from "../../lib/api/settings";
  import IntegrationForm from "./IntegrationForm.svelte";

  let integrations: IntegrationView[] = $state([]);
  /** The integration in the form: its id, 0 for a new one, null for none. */
  let open: number | null = $state(null);
  let problem = $state("");
  let busy = $state(false);

  let opened = $derived(integrations.find((integration) => integration.id === open) ?? null);

  async function reload() {
    try {
      integrations = await integrationList();
    } catch (error) {
      problem = sentence(error);
    }
  }

  async function activate(id: number) {
    problem = "";
    busy = true;

    try {
      await integrationActivate(id);
      await reload();
    } catch (error) {
      problem = sentence(error);
    } finally {
      busy = false;
    }
  }

  async function saved(id: number) {
    open = id;
    await reload();
  }

  onMount(reload);
</script>

{#if problem}<p class="problem">{problem}</p>{/if}

<section class="card">
  <h2>Integrations</h2>
  <table class="data">
    <thead><tr><th>Name</th><th>Model</th><th>Key</th><th></th></tr></thead>
    <tbody>
      {#each integrations as integration (integration.id)}
        <tr>
          <td class="wrap">
            {integration.name}
            {#if integration.active}<span class="badge success">active</span>{/if}
          </td>
          <td class="wrap mono">{integration.model}</td>
          <td>
            {#if integration.hasKey}
              <span class="badge success">stored</span>
            {:else}
              <span class="badge warning">none</span>
            {/if}
          </td>
          <td class="actions">
            <button type="button" onclick={() => (open = integration.id)}>Open</button>
            {#if !integration.active}
              <button type="button" disabled={busy} onclick={() => activate(integration.id)}>Make active</button>
            {/if}
          </td>
        </tr>
      {:else}
        <tr><td colspan="4" class="note">No integration yet: add the first one.</td></tr>
      {/each}
    </tbody>
  </table>
  <p><button type="button" onclick={() => (open = 0)}>A new integration</button></p>
</section>

{#if open === 0}
  {#key open}
    <IntegrationForm integration={null} onsaved={saved} onclose={() => (open = null)} />
  {/key}
{:else if opened !== null}
  {#key opened.id}
    <IntegrationForm integration={opened} onsaved={saved} onclose={() => (open = null)} />
  {/key}
{/if}

<style>
  table.data {
    width: 100%;
  }

  td.actions {
    white-space: nowrap;
  }

  td.actions button + button {
    margin-left: 0.4rem;
  }
</style>
