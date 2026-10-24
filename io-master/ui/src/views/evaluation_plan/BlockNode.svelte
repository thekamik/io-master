<!--
  A block of the scheme of a task on the drawing (TaskScheme.svelte), drawn
  by Svelte Flow: its kind and name; the requirement also its verdict and
  the four text results it gives to the outputs (empty in the plan, filled
  in the Evaluation lab). An input has a handle on its right (it
  goes to the requirement), an output one on its left (it comes from the
  requirement), the requirement both. Which blocks may be connected is
  decided by the program (model/task_scheme.rs), which refuses the rest.
  Presentation only.
-->
<script lang="ts">
  import { Handle, Position, type NodeProps } from "@xyflow/svelte";

  import type { BlockView } from "../../lib/api/evaluation_plan";

  let { data, selected }: NodeProps & { data: { block: BlockView; label: string; title: string; results: string[] } } = $props();

  const titles = { input: "Input", requirement: "Requirement", output: "Output" };
  const conditions = { always: "always", not_met: "only when not met", met: "only when met" };

  let block = $derived(data.block);
</script>

<div class="tile {block.kind}" class:selected>
  {#if block.kind !== "input"}<Handle type="target" position={Position.Left} />{/if}

  <div class="tile-head"><span class="tile-kind">{titles[block.kind]}</span></div>
  {#if block.kind === "requirement"}
    <div class="tile-id">{data.label}</div>
    <div class="tile-title">{data.title}</div>
    <div class="tile-verdict">met / not met: proposed by the AI, decided by the evaluator</div>
    <ul class="tile-results">
      {#each data.results as result (result)}<li>{result}</li>{/each}
    </ul>
  {:else}
    <div class="tile-title">{block.name}</div>
  {/if}
  {#if block.kind === "output"}
    <div class="tile-id">{block.format === "latex" ? "LaTeX" : "text"} · {conditions[block.condition]}</div>
  {/if}

  {#if block.kind !== "output"}<Handle type="source" position={Position.Right} />{/if}
</div>
