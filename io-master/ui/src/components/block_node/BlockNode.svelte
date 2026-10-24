<!--
  The component "block of a scheme": a block of the scheme of a task on the
  drawing, drawn by Svelte Flow, in the Evaluation plan (TaskScheme.svelte)
  and the Evaluation lab (LabScheme.svelte): its kind and name; the requirement also its verdict and
  the four text results it gives to the outputs (empty in the plan, filled
  in the Evaluation lab). A block the AI only proposed is dashed; a block
  with a correction of the AI waiting is marked. An input has a handle on its right (it
  goes to the requirement), an output one on its left (it comes from the
  requirement), the requirement both. Which blocks may be connected is
  decided by the program (model/task_scheme.rs), which refuses the rest.
  Presentation only.
-->
<script lang="ts">
  import { Handle, Position, type NodeProps } from "@xyflow/svelte";

  import type { BlockNodeData } from "../../lib/api/components/block_node";

  let { data, selected }: NodeProps & { data: BlockNodeData } = $props();

  const titles = { input: "Input", requirement: "Requirement", output: "Output" };
  const conditions = { always: "always", not_met: "only when not met", met: "only when met" };

  let block = $derived(data.block);
</script>

<div class="tile {block.kind}" class:selected class:proposed={block.proposal != null}>
  {#if block.kind !== "input"}<Handle type="target" position={Position.Left} />{/if}

  <div class="tile-head">
    <span class="tile-kind">{titles[block.kind]}</span>
    {#if block.proposal != null}<span class="tile-ai">proposed by the AI</span>{/if}
    {#if block.correction}<span class="tile-ai">correction of the AI</span>{/if}
  </div>
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

  {#if data.note}<div class="tile-note">{data.note}</div>{/if}

  {#if block.kind !== "output"}<Handle type="source" position={Position.Right} />{/if}
</div>
