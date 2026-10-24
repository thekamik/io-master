<!--
  The element "screen of a task" of the view Evaluation lab (its commands in
  src/views/evaluation_lab/evaluation_lab.rs, step 52): the scheme of the
  task read only (LabScheme) and, beside it, the panel of the block chosen
  (InputPanel, RequirementPanel, OutputPanel), the requirement as the
  standard writes it and its related elements. Every command answers with
  the whole screen, so the page always shows what the program keeps.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import PartRows from "../../components/part_rows/PartRows.svelte";
  import { sentence } from "../../lib/api/errors";
  import * as evaluationLab from "../../lib/api/evaluation_lab";
  import InputPanel from "./InputPanel.svelte";
  import LabScheme from "./LabScheme.svelte";
  import OutputPanel from "./OutputPanel.svelte";
  import RequirementPanel from "./RequirementPanel.svelte";

  let { evaluation, task, onclose }: { evaluation: string; task: string; onclose: () => void } = $props();

  let screen = $state<evaluationLab.LabTaskScreenView | null>(null);
  let chosen: string | null = $state(null);
  let problem = $state("");

  let block = $derived(screen?.blocks.find((candidate) => candidate.uuid === chosen) ?? null);
  let side: HTMLElement | undefined = $state();

  // Another block chosen: its panel is read from the top.
  $effect(() => {
    void chosen;
    side?.scrollTo({ top: 0 });
  });

  onMount(async () => {
    try {
      screen = await evaluationLab.labTaskGet(evaluation, task);
      chosen = screen.blocks.find((candidate) => candidate.kind === "requirement")?.uuid ?? null;
    } catch (error) {
      problem = sentence(error);
    }
  });

  /** The screen the program gave back after a change. */
  function show(next: evaluationLab.LabTaskScreenView) {
    screen = next;
  }
</script>

<div class="editor-head">
  <button type="button" class="small" onclick={onclose}>← The evaluation</button>
  {#if screen}<h2><span class="object-id">{screen.requirement.label}</span> {screen.requirement.title}</h2>{/if}
</div>

{#if problem}<p class="problem">{problem}</p>{/if}

{#if screen}
  <div class="scheme-body">
    <div class="scheme-canvas">
      <LabScheme {screen} {chosen} onchoose={(uuid) => (chosen = uuid)} />
    </div>
    <div class="scheme-side" bind:this={side}>
      {#if block?.kind === "input"}
        <div class="card"><InputPanel {evaluation} {screen} {block} onchanged={show} /></div>
      {:else if block?.kind === "requirement"}
        <div class="card"><RequirementPanel {evaluation} {screen} onchanged={show} /></div>
      {:else if block?.kind === "output"}
        <div class="card"><OutputPanel {evaluation} {screen} {block} onchanged={show} /></div>
      {:else}
        <p class="note">Choose a block of the scheme.</p>
      {/if}
      <div class="card">
        <h3>The standard</h3>
        <h4><span class="object-id">{screen.requirement.label}</span> {screen.requirement.title}</h4>
        <p class="note">{screen.requirement.catalog_title}</p>
        <PartRows rows={screen.requirement.parts} />
        {#if screen.related.length > 0}
          <h4>Related elements</h4>
          {#each screen.related as element, index (index)}
            <p><span class="object-id">{element.label}</span> {element.title}</p>
          {/each}
        {/if}
      </div>
    </div>
  </div>
{/if}
