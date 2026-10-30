<!--
  One evaluation in the panel of the evaluation: two tabs (user,
  2026-10-30), Inputs (EvaluationInputs: its identification, its agent, its
  documents and proofs) and Work units (EvaluationRun: the assessment by the
  AI; EvaluationUnits: the list in the order of the work; a work unit opened
  shows EvaluationUnit). The assessment stays shown while a work unit is
  open, so that its progress is never lost. Which tab, page and work unit
  are shown is presentation only: it stays in the page.
-->
<script lang="ts">
  import EvaluationInputs from "./EvaluationInputs.svelte";
  import EvaluationRun from "./EvaluationRun.svelte";
  import EvaluationUnit from "./EvaluationUnit.svelte";
  import EvaluationUnits from "./EvaluationUnits.svelte";

  let { evaluation, onback }: { evaluation: number; onback: () => void } = $props();

  let tab: "inputs" | "units" = $state("inputs");
  let openUnit: number | null = $state(null);
  /** The work units in the order of the work, for "previous" and "next". */
  let order: number[] = $state([]);
  /** Counts the work units written by the AI, so that the list is read again. */
  let changes = $state(0);
  let page = $state(0);
  let filter = $state("");
  let verdict = $state("");
</script>

<p><button type="button" class="ghost" onclick={onback}>← Evaluations</button></p>

<div class="tabs">
  <button type="button" class:active={tab === "inputs"} onclick={() => (tab = "inputs")}>Inputs</button>
  <button type="button" class:active={tab === "units"} onclick={() => (tab = "units")}>Work units</button>
</div>

{#if tab === "inputs"}
  <EvaluationInputs {evaluation} />
{:else}
  <EvaluationRun {evaluation} onstep={() => (changes += 1)} />

  {#if openUnit !== null}
    {#key openUnit}
      <EvaluationUnit
        {evaluation}
        unit={openUnit}
        {order}
        onopen={(id) => (openUnit = id)}
        onback={() => (openUnit = null)}
      />
    {/key}
  {:else}
    <EvaluationUnits
      {evaluation}
      {changes}
      bind:page
      bind:filter
      bind:verdict
      onopen={(id, ids) => {
        order = ids;
        openUnit = id;
      }}
    />
  {/if}
{/if}
