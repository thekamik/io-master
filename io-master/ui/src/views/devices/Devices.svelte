<!--
  The view Devices (src/views/devices/): the settings of the scanner
  above the table of ports. The view holds no data of its own: each element reads
  its own, and the view only passes on that the settings were saved, so
  that the table scans again with them.
-->
<script lang="ts">
  import PortsTable from "./PortsTable.svelte";
  import ScannerSettings from "./ScannerSettings.svelte";

  let table: PortsTable | null = $state(null);
</script>

<ScannerSettings onsaved={() => table?.scan()} />
<PortsTable bind:this={table} />
