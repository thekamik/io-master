<!--
  The element "scanner settings" of the view Devices: which kinds of
  port a scan looks for and which columns the table shows, folded under a
  one-line summary. Its commands: scanner_settings_get and
  scanner_settings_save (src/views/devices/devices.rs); its data:
  src/views/devices/scanner_settings/view.rs.

  After a save it tells the view (onsaved), which asks the table of
  ports to scan again.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../../lib/api/errors";
  import * as devices from "../../lib/api/devices";

  let { onsaved }: { onsaved: () => void } = $props();

  let settings: devices.ScannerSettingsView | null = $state(null);
  let problem = $state("");
  let busy = $state(false);

  // The choice in the form, filled from the stored settings.
  let allTypes = $state(true);
  let types: string[] = $state([]);
  let columns: string[] = $state([]);

  function show(next: devices.ScannerSettingsView) {
    settings = next;
    allTypes = next.all_types;
    types = [...next.types];
    columns = [...next.columns];
  }

  async function load() {
    try {
      show(await devices.scannerSettingsGet());
    } catch (error) {
      problem = `Your settings cannot be read: ${sentence(error)}`;
    }
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    busy = true;
    problem = "";

    try {
      show(await devices.scannerSettingsSave(allTypes, allTypes ? [] : types, columns));
      onsaved();
    } catch (error) {
      problem = `Your settings were not saved: ${sentence(error)}`;
    } finally {
      busy = false;
    }
  }

  onMount(load);
</script>

{#if problem}<p class="problem">{problem}</p>{/if}

{#if settings}
  <details class="settings">
    <summary>Scanner settings: {settings.summary}</summary>

    <form onsubmit={save}>
      <fieldset>
        <legend>Kinds of port</legend>
        <label class="choice"><input type="checkbox" bind:checked={allTypes} /> All kinds</label>
        <div class="choices">
          {#each settings.choices.types as name (name)}
            <label class="choice">
              <input type="checkbox" bind:group={types} value={name} disabled={allTypes} />
              {name}
            </label>
          {/each}
        </div>
      </fieldset>

      <fieldset>
        <legend>Columns of the table</legend>
        <div class="choices">
          {#each settings.choices.columns as column (column.key)}
            <label class="choice"><input type="checkbox" bind:group={columns} value={column.key} /> {column.title}</label>
          {/each}
        </div>
      </fieldset>

      <button type="submit" disabled={busy}>Save and scan</button>
    </form>
  </details>
{/if}
