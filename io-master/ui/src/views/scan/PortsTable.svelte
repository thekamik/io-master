<!--
  The element "table of ports" of the view Scan
  (src/views/scan/ports_table/): the serial ports of the computer, with
  the columns of the settings.

  It scans when it is shown, when "Scan again" is pressed, and when the
  view asks for it after the settings were saved (scan(), below). The
  scan only reads the descriptions of the ports; it never opens one.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../../lib/api/errors";
  import * as portsTable from "../../lib/api/scan/ports_table";

  let table: portsTable.PortsTableView | null = $state(null);
  let problem = $state("");
  let busy = $state(false);

  /** Scans again; called by the view after the settings were saved. */
  export async function scan() {
    busy = true;
    problem = "";

    try {
      table = await portsTable.scan();
    } catch (error) {
      problem = sentence(error);
    } finally {
      busy = false;
    }
  }

  onMount(scan);
</script>

<section>
  <h2>Serial ports <button class="small" onclick={scan} disabled={busy}>{busy ? "Scanning…" : "Scan again"}</button></h2>

  {#if problem}
    <p class="problem">{problem}</p>
  {:else if table?.problem}
    <p class="problem">{table.problem}</p>
  {:else if table && table.rows.length === 0}
    <p>No serial ports of the chosen kinds were found.</p>
  {:else if table}
    <p class="note">{table.rows.length} port{table.rows.length === 1 ? "" : "s"} found.</p>
    <div class="table-card">
      <table class="ports">
        <thead>
          <tr>
            {#each table.columns as column (column.key)}<th>{column.title}</th>{/each}
          </tr>
        </thead>
        <tbody>
          {#each table.rows as row, index (index)}
            <tr>
              {#each row as value, cell (cell)}<td>{value}</td>{/each}
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {:else if busy}
    <p class="note">Scanning…</p>
  {/if}
</section>
