<!--
  A tile of another scheme of the test project, drawn on the canvas of
  a scheme because a link joins it to a tile there (step 26 of
  docs/todo.md): grey and dashed, read only, with the number and name
  of its scheme. A click on it opens that scheme (SchemeEditor.svelte).

  It has both handles, so that a link can also be drawn to it by
  dragging; the program decides whether the link is allowed.
-->
<script lang="ts">
  import { Handle, Position, type NodeProps } from "@xyflow/svelte";

  import { schemeName, type ShadowView } from "../lib/api";

  let { data }: NodeProps & { data: { shadow: ShadowView } } = $props();

  const titles = { input: "Input", requirement: "Requirement", output: "Output" };

  let shadow = $derived(data.shadow);
</script>

<div class="tile shadow {shadow.kind}" title="Open the scheme {schemeName({ number: shadow.scheme_number, name: shadow.scheme_name })}">
  <Handle type="target" position={Position.Left} />

  <div class="tile-head">
    <span class="tile-kind">{titles[shadow.kind]}</span>
    <span class="tile-flag">other scheme</span>
  </div>
  {#if shadow.identifier}<div class="tile-id">{shadow.identifier}</div>{/if}
  <div class="tile-title">{shadow.title}</div>
  <div class="tile-external">in {schemeName({ number: shadow.scheme_number, name: shadow.scheme_name })}</div>

  {#if shadow.kind !== "output"}
    <Handle type="source" position={Position.Right} />
  {/if}
</div>
