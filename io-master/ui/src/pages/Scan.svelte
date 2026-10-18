<!--
  The scan page: the serial ports of the computer, and the settings
  of the scanner, folded under a one-line summary.

  The scan only reads the descriptions of the ports; it never opens
  one.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../lib/api/errors";
import { saveScanSettings, scanPorts, type ScanView } from "../lib/api/scan";

  let view: ScanView | null = $state(null);
  let problem = $state("");
  let busy = $state(false);

  // The choice in the form, filled from the stored settings.
  let allTypes = $state(true);
  let types: string[] = $state([]);
  let columns: string[] = $state([]);

  function show(next: ScanView) {
    view = next;
    allTypes = next.settings.all_types;
    types = [...next.settings.types];
    columns = [...next.settings.columns];
  }

  async function scan() {
    busy = true;
    problem = "";

    try {
      show(await scanPorts());
    } catch (error) {
      problem = sentence(error);
    } finally {
      busy = false;
    }
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    busy = true;
    problem = "";

    try {
      show(await saveScanSettings(allTypes, allTypes ? [] : types, columns));
    } catch (error) {
      problem = `Your settings were not saved: ${sentence(error)}`;
    } finally {
      busy = false;
    }
  }

  onMount(scan);
</script>

{#if problem}<p class="problem">{problem}</p>{/if}

{#if view}
  <details class="settings">
    <summary>Scanner settings: {view.settings.summary}</summary>

    <form onsubmit={save}>
      <fieldset>
        <legend>Kinds of port</legend>
        <label class="choice"><input type="checkbox" bind:checked={allTypes} /> All kinds</label>
        <div class="choices">
          {#each view.choices.types as name (name)}
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
          {#each view.choices.columns as column (column.key)}
            <label class="choice"><input type="checkbox" bind:group={columns} value={column.key} /> {column.title}</label>
          {/each}
        </div>
      </fieldset>

      <button type="submit" disabled={busy}>Save and scan</button>
    </form>
  </details>

  <section>
    <h2>Serial ports <button class="small" onclick={scan} disabled={busy}>{busy ? "Scanning…" : "Scan again"}</button></h2>

    {#if view.problem}
      <p class="problem">{view.problem}</p>
    {:else if view.table && view.table.rows.length === 0}
      <p>No serial ports of the chosen kinds were found.</p>
    {:else if view.table}
      <p class="note">{view.table.rows.length} port{view.table.rows.length === 1 ? "" : "s"} found.</p>
      <div class="table-card">
      <table class="ports">
        <thead>
          <tr>
            {#each view.table.columns as column (column.key)}<th>{column.title}</th>{/each}
          </tr>
        </thead>
        <tbody>
          {#each view.table.rows as row, index (index)}
            <tr>
              {#each row as value, cell (cell)}<td>{value}</td>{/each}
            </tr>
          {/each}
        </tbody>
      </table>
      </div>
    {/if}
  </section>
{:else if busy}
  <p class="note">Scanning…</p>
{/if}
