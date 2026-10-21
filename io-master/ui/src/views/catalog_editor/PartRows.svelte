<!--
  The parts of a group or a control (statement, items, guidance, ...), as
  the rows the program gives with their level: shown indented, without
  recursion. Presentation only; no command.
-->
<script lang="ts">
  import type { PartRowView } from "../../lib/api/catalog_editor/part_rows";

  let { rows }: { rows: PartRowView[] } = $props();
</script>

{#each rows as part, index (part.id ?? index)}
  <div class="oscal-part" style="margin-left: {0.6 + part.level * 1.2}rem">
    <span class="note">{part.name}{part.id ? ` · ${part.id}` : ""}{part.title ? ` · ${part.title}` : ""}</span>
    {#if part.prose}<p>{part.prose}</p>{/if}
  </div>
{/each}
