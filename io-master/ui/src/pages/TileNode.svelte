<!--
  A tile on the canvas of a scheme (SchemeEditor.svelte), drawn by
  Svelte Flow: its kind, identifier and title, whether it is only
  proposed by the AI (a dashed frame), and how many findings of the
  checks it has.

  Links are drawn from the handle on the right of a tile to the
  handle on the left of another; which kinds may be linked is decided
  by the program (src/evaluation/scheme.rs), which refuses the rest.
-->
<script lang="ts">
  import { Handle, Position, type NodeProps } from "@xyflow/svelte";

  import type { TileView } from "../lib/api";

  let { data, selected }: NodeProps & { data: { tile: TileView; findings: number } } = $props();

  const titles = { input: "Input", requirement: "Requirement", output: "Output" };

  let tile = $derived(data.tile);
</script>

<div class="tile {tile.kind}" class:proposed={tile.state === "proposed"} class:selected>
  <!-- Every tile can be the end of a link: an input of the requirement that provides it, a requirement of the one that precedes it. -->
  <Handle type="target" position={Position.Left} />

  <div class="tile-head">
    <span class="tile-kind">{titles[tile.kind]}</span>
    {#if tile.state === "proposed"}<span class="tile-flag">AI</span>{/if}
    {#if data.findings > 0}<span class="tile-flag warn" title="Findings of the checks">{data.findings}</span>{/if}
  </div>
  {#if tile.identifier}<div class="tile-id">{tile.identifier}</div>{/if}
  <div class="tile-title">{tile.title}</div>
  {#if tile.external}<div class="tile-external">from {tile.external.document}</div>{/if}

  {#if tile.kind !== "output"}
    <Handle type="source" position={Position.Right} />
  {/if}
</div>
