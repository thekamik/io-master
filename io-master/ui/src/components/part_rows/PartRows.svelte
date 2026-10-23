<!--
  The component "parts of an object" (src/components/part_rows/): the
  parts of a group or a control (statement, items, guidance, ...), as the
  rows the program gives with their level: shown indented, without
  recursion. Presentation only; no command. Used by the object chosen and
  by the proposals of the editor of a catalog.
-->
<script lang="ts">
  import type { PartRowView } from "../../lib/api/components/part_rows";

  let { rows }: { rows: PartRowView[] } = $props();
</script>

{#each rows as part, index (part.id ?? index)}
  <div class="oscal-part" style="margin-left: {0.6 + part.level * 1.2}rem">
    <span class="note">{part.name}{part.id ? ` · ${part.id}` : ""}{part.title ? ` · ${part.title}` : ""}</span>
    {#if part.prose}<p>{part.prose}</p>{/if}
  </div>
{/each}
